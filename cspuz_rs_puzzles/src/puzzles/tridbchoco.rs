use crate::util::{self, Grid};
use cspuz_rs::serializer::{
    problem_to_url_with_context, url_to_problem, Choice, Combinator, Context, ContextBasedGrid,
    Dict, HexInt, MultiDigit, Optionalize, Size, Spaces, Tuple2,
};
use cspuz_rs::solver::{BoolVar, Solver};

use cspuz_core::custom_constraints::SimpleCustomConstraint;

/// Triangular Double Choco:
/// 三角形格子の盤面をブロックに分割する。各ブロックは白と灰色の
/// カタマリを1つずつ含み、その2つは回転/鏡映を除いて合同。
/// 数字はその数字の入ったカタマリの大きさを表す。
///
/// 格子: セル (x,y) は x+y が偶数なら△、奇数なら▽。
/// 隣接: 左右は常に隣接。上下は△なら下、▽なら上とだけ隣接する。
///
/// 盤面の形: 頂点が上向きの大きな正三角形。盤面サイズ(h,w)から
/// 三角形の領域を決める。頂点のセルは△(x+yが偶数)になるように、
/// 頂点の列apexは偶数に丸める。白と灰色の総数が一致する必要が
/// あるため、三角形の行数rowsは偶数に丸める。
fn tri_region(h: usize, w: usize) -> (usize, usize) {
    let mut apex = (w - 1) / 2;
    if apex % 2 == 1 {
        apex -= 1;
    }
    let mut rows = h.min(apex + 1).min(w - apex);
    if rows % 2 == 1 {
        rows -= 1;
    }
    (apex, rows)
}

fn tri_in_region(h: usize, w: usize, y: usize, x: usize) -> bool {
    let (apex, rows) = tri_region(h, w);
    y < rows && x + y >= apex && x <= apex + y
}

pub fn solve_tridbchoco(
    color: &[Vec<i32>],
    num: &[Vec<Option<i32>>],
) -> Option<(Vec<Vec<Option<bool>>>, Vec<Vec<Option<bool>>>)> {
    let (h, w) = util::infer_shape(color);
    assert_eq!(util::infer_shape(num), (h, w));

    let mut solver = Solver::new();
    let borders = TriBorders::new(&mut solver, h, w);
    solver.add_answer_key_bool(&borders.vars);

    let cell_num = Grid::from_vecs(
        &(num
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&n| match n {
                        Some(-1) => None,
                        Some(n) => Some(n as usize),
                        None => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()),
    );

    let constraint = TridbchocoConstraint {
        height: h,
        width: w,
        cell_color: Grid::from_vecs(color),
        cell_num,
        decision_stack: vec![],
        border_values: vec![None; borders.vars.len()],
    };

    // 事前チェック: 各ブロックは白と灰色のカタマリを1つずつ、同じ
    // 大きさで含むので、盤内の灰色マスの数は盤内の全マスの数の
    // ちょうど半分でなければならない。
    let (_, region_rows) = tri_region(h, w);
    let mut grey_count = 0;
    for y in 0..h {
        for x in 0..w {
            if tri_in_region(h, w, y, x) && color[y][x] == 1 {
                grey_count += 1;
            }
        }
    }
    if grey_count * 2 != region_rows * region_rows {
        return None;
    }

    solver.add_custom_constraint(Box::new(constraint), borders.vars.clone());

    solver
        .irrefutable_facts()
        .map(|f| borders.extract(&f))
}

/// 三角形格子の実際の境界線の変数群。
/// 左右の境界線はすべて実在する。上下の境界線は上側のセルが△の
/// ときだけ実在する。
struct TriBorders {
    vars: Vec<BoolVar>,
    height: usize,
    width: usize,
    hor: Grid<usize>, // (y, x) と (y, x+1) の間
    ver: Grid<usize>, // (y, x) と (y+1, x) の間
}

impl TriBorders {
    fn new(solver: &mut Solver, height: usize, width: usize) -> TriBorders {
        let mut vars = vec![];
        let mut hor = Grid::new(height, width - 1, !0);
        let mut ver = Grid::new(height - 1, width, !0);

        for y in 0..height {
            for x in 0..(width - 1) {
                hor[(y, x)] = vars.len();
                vars.push(solver.bool_var());
            }
        }
        for y in 0..(height - 1) {
            for x in 0..width {
                if (x + y) % 2 == 0 {
                    ver[(y, x)] = vars.len();
                    vars.push(solver.bool_var());
                }
            }
        }

        TriBorders {
            vars,
            height,
            width,
            hor,
            ver,
        }
    }

    fn extract(
        &self,
        f: &cspuz_rs::solver::OwnedPartialModel,
    ) -> (Vec<Vec<Option<bool>>>, Vec<Vec<Option<bool>>>) {
        let mut hor = vec![vec![None; self.width - 1]; self.height];
        let mut ver = vec![vec![None; self.width]; self.height - 1];
        for y in 0..self.height {
            for x in 0..(self.width - 1) {
                hor[y][x] = f.get(&self.vars[self.hor[(y, x)]]);
            }
        }
        for y in 0..(self.height - 1) {
            for x in 0..self.width {
                let v = self.ver[(y, x)];
                if v != !0 {
                    ver[y][x] = f.get(&self.vars[v]);
                }
            }
        }
        (hor, ver)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BorderState {
    Undecided,
    Wall,
    Connected,
}

struct TridbchocoConstraint {
    height: usize,
    width: usize,
    cell_color: Grid<i32>,
    cell_num: Grid<Option<usize>>,
    decision_stack: Vec<usize>,
    border_values: Vec<Option<bool>>,
}

impl TridbchocoConstraint {
    fn hor_var(&self, y: usize, x: usize) -> usize {
        y * (self.width - 1) + x
    }

    fn ver_var(&self, y: usize, x: usize) -> usize {
        assert!((x + y) % 2 == 0);
        self.height * (self.width - 1) + {
            let mut idx = 0;
            for yy in 0..y {
                for xx in 0..self.width {
                    if (xx + yy) % 2 == 0 {
                        idx += 1;
                    }
                }
            }
            for xx in 0..x {
                if (xx + y) % 2 == 0 {
                    idx += 1;
                }
            }
            idx
        }
    }

    fn tri_neighbors(&self, y: usize, x: usize) -> Vec<(usize, usize)> {
        let (h, w) = (self.height, self.width);
        let mut ret = vec![];
        if x > 0 && tri_in_region(h, w, y, x - 1) {
            ret.push((y, x - 1));
        }
        if x + 1 < w && tri_in_region(h, w, y, x + 1) {
            ret.push((y, x + 1));
        }
        if (x + y) % 2 == 0 {
            if y + 1 < h && tri_in_region(h, w, y + 1, x) {
                ret.push((y + 1, x));
            }
        } else if y > 0 && tri_in_region(h, w, y - 1, x) {
            ret.push((y - 1, x));
        }
        ret
    }

    fn border_state(&self, y: usize, x: usize, y2: usize, x2: usize) -> BorderState {
        let idx = if y == y2 {
            self.hor_var(y, x.min(x2))
        } else {
            self.ver_var(y.min(y2), x)
        };
        match self.border_values[idx] {
            None => BorderState::Undecided,
            Some(true) => BorderState::Wall,
            Some(false) => BorderState::Connected,
        }
    }

    fn current_reason(&self) -> Vec<(usize, bool)> {
        self.decision_stack
            .iter()
            .map(|&i| (i, self.border_values[i].unwrap()))
            .collect()
    }

    // 連結成分を計算する (potential: Undecided も接続とみなす)
    fn compute_components(
        &self,
        ignore_color: bool,
        is_potential: bool,
    ) -> (Grid<usize>, Vec<Vec<(usize, usize)>>) {
        let (h, w) = (self.height, self.width);
        let mut group_id = Grid::new(h, w, !0usize);
        let mut components = vec![];
        let mut last_id = 0usize;

        for y in 0..h {
            for x in 0..w {
                if !tri_in_region(h, w, y, x) {
                    group_id[(y, x)] = usize::MAX - 1;
                    continue;
                }
                if group_id[(y, x)] != !0 {
                    continue;
                }
                let mut cells = vec![];
                let mut stack = vec![(y, x)];
                group_id[(y, x)] = last_id;
                while let Some((y, x)) = stack.pop() {
                    cells.push((y, x));
                    for &(y2, x2) in &self.tri_neighbors(y, x) {
                        if !ignore_color && self.cell_color[(y, x)] != self.cell_color[(y2, x2)] {
                            continue;
                        }
                        let border = self.border_state(y, x, y2, x2);
                        if border == BorderState::Connected
                            || (is_potential && border == BorderState::Undecided)
                        {
                            if group_id[(y2, x2)] == !0 {
                                group_id[(y2, x2)] = last_id;
                                stack.push((y2, x2));
                            }
                        }
                    }
                }
                components.push(cells);
                last_id += 1;
            }
        }
        (group_id, components)
    }
}

impl SimpleCustomConstraint for TridbchocoConstraint {
    fn lazy_propagation(&self) -> bool {
        false
    }

    fn initialize_sat(&mut self, num_inputs: usize) {
        assert_eq!(num_inputs, self.border_values.len());
    }

    fn notify(&mut self, index: usize, value: bool) {
        if self.border_values[index].is_none() {
            self.border_values[index] = Some(value);
            self.decision_stack.push(index);
        }
    }

    fn undo(&mut self) {
        if let Some(top) = self.decision_stack.pop() {
            self.border_values[top] = None;
        }
    }

    fn find_inconsistency(&mut self) -> Option<Vec<(usize, bool)>> {
        let (h, w) = (self.height, self.width);

        let (_unit_id, units) = self.compute_components(false, false);
        let (punit_id, punits) = self.compute_components(false, true);
        let (block_id, blocks) = self.compute_components(true, false);

        // 潜在ユニット同士の隣接関係
        let mut punit_adj = vec![vec![]; punits.len()];
        for y in 0..h {
            for x in 0..w {
                if !tri_in_region(h, w, y, x) {
                    continue;
                }
                for &(y2, x2) in &self.tri_neighbors(y, x) {
                    let i = punit_id[(y, x)];
                    let j = punit_id[(y2, x2)];
                    if i != j {
                        punit_adj[i].push(j);
                        punit_adj[j].push(i);
                    }
                }
            }
        }
        for adj in &mut punit_adj {
            adj.sort();
            adj.dedup();
        }

        for block in &blocks {
            let mut punit_by_color = [!0usize, !0usize];
            let mut size_by_color = [0usize, 0usize];
            let mut has_num = [false, false];
            let mut num: Option<usize> = None;

            for &(y, x) in block {
                let c = self.cell_color[(y, x)] as usize;
                let pid = punit_id[(y, x)];
                if punit_by_color[c] == !0 {
                    punit_by_color[c] = pid;
                } else if punit_by_color[c] != pid {
                    return Some(self.current_reason());
                }
                size_by_color[c] += 1;
                if let Some(n) = self.cell_num[(y, x)] {
                    has_num[c] = true;
                    match num {
                        Some(m) if m != n => return Some(self.current_reason()),
                        _ => num = Some(n),
                    }
                }
            }

            // 潜在ユニットが相手の色の大きさに到達できない
            if punit_by_color[0] != !0 && punits[punit_by_color[0]].len() < size_by_color[1] {
                return Some(self.current_reason());
            }
            if punit_by_color[1] != !0 && punits[punit_by_color[1]].len() < size_by_color[0] {
                return Some(self.current_reason());
            }

            if let Some(n) = num {
                // 確定済みのカタマリが数字より大きい
                if n < size_by_color[0] || n < size_by_color[1] {                    return Some(self.current_reason());
                }
                // 潜在ユニットが数字に到達できない
                for c in 0..2 {
                    if punit_by_color[c] != !0 && n > punits[punit_by_color[c]].len() {
                        if has_num[c] || size_by_color[1 - c] > 0 {
                            return Some(self.current_reason());
                        }
                    }
                }
            }
        }

        // 同じブロック内の隣接セル間の壁
        for y in 0..h {
            for x in 0..w {
                if !tri_in_region(h, w, y, x) {
                    continue;
                }
                for &(y2, x2) in &self.tri_neighbors(y, x) {
                    if block_id[(y, x)] == block_id[(y2, x2)]
                        && self.border_state(y, x, y2, x2) == BorderState::Wall
                    {
                        return Some(self.current_reason());
                    }
                }
            }
        }

        // 合同性: 各ユニットは隣接する反対色の潜在ユニットに
        // (回転6種×鏡映のいずれかで) 収まらなければならない
        for unit in &units {
            let (y0, x0) = unit[0];
            let color = self.cell_color[(y0, x0)];
            let pid = punit_id[(y0, x0)];

            let mut ok = false;
            'outer: for &other_pid in &punit_adj[pid] {
                let (oy, ox) = punits[other_pid][0];
                if self.cell_color[(oy, ox)] == color {
                    continue;
                }
                if punits[other_pid].len() < unit.len() {
                    continue;
                }
                if is_congruent_into(unit, &punits[other_pid]) {
                    ok = true;
                    break 'outer;
                }
            }
            if !ok {
                return Some(self.current_reason());
            }
        }

        None
    }
}

// 三角形格子(セルの隣接グラフ)の合同変換 12種。
// (x,y) -> ((a*x+b*y+tx)/2, (c*x+d*y+ty)/2)
// 平行移動(tx,ty)は x+y の偶奇(セルの向き)によって異なる。
fn tri_apply_transform(
    p: (i32, i32),
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    t_up: (i32, i32),
    t_down: (i32, i32),
) -> (i32, i32) {
    // p は (y, x) の順。変換の係数は (x, y) 座標系で定義されているので、
    // 入れ替えて計算し、結果も (y, x) の順で返す
    let (y, x) = p;
    let t = if (x + y) % 2 == 0 { t_up } else { t_down };
    let x2 = (a * x + b * y + t.0) / 2;
    let y2 = (c * x + d * y + t.1) / 2;
    (y2, x2)
}

fn tri_apply_map(i: usize, p: (i32, i32)) -> (i32, i32) {
    let maps: [(i32, i32, i32, i32, (i32, i32), (i32, i32)); 12] = [
        (2, 0, 0, 2, (0, 0), (0, 0)), // 恒等変換
        (-2, 0, 0, 2, (0, 0), (0, 0)), // 鏡映
        (-1, -3, 1, -1, (0, 0), (1, 1)), // 120°回転系
        (-1, 3, -1, -1, (0, 0), (-1, 1)),
        (1, -3, -1, -1, (0, 0), (1, 1)),
        (1, 3, 1, -1, (0, 0), (-1, 1)),
        (-2, 0, 0, -2, (0, 2), (0, 2)), // 180°回転
        (-1, -3, -1, 1, (0, 2), (1, 1)), // 60°回転系
        (-1, 3, 1, 1, (0, -2), (-1, -3)),
        (1, -3, 1, 1, (0, 2), (1, 1)),
        (1, 3, -1, 1, (0, -2), (-1, -3)),
        (2, 0, 0, -2, (0, 2), (0, 2)), // 鏡映
    ];
    let (a, b, c, d, t_up, t_down) = maps[i];
    tri_apply_transform(p, a, b, c, d, t_up, t_down)
}

// 12変換のいずれかで a のセルがすべて b のセルに収まるか
fn is_congruent_into(a: &[(usize, usize)], b: &[(usize, usize)]) -> bool {
    if b.len() < a.len() {
        return false;
    }
    if a.len() <= 1 {
        return true;
    }

    let b_set: std::collections::HashSet<(i32, i32)> = b
        .iter()
        .map(|&(y, x)| (y as i32, x as i32))
        .collect();
    let pts: Vec<(i32, i32)> = a.iter().map(|&(y, x)| (y as i32, x as i32)).collect();

    for i in 0..12 {
        let transformed: Vec<(i32, i32)> = pts.iter().map(|&p| tri_apply_map(i, p)).collect();
        let mut min_y = i32::MAX;
        let mut min_x = i32::MAX;
        for &(y, x) in &transformed {
            min_y = min_y.min(y);
            min_x = min_x.min(x);
        }
        let norm: Vec<(i32, i32)> = transformed
            .iter()
            .map(|&(y, x)| (y - min_y, x - min_x))
            .collect();
        // 正規化した形を b の中のどこかに重ねられるか。
        // 基準点 norm[0] が b の各点に重なる平行移動をすべて試す。
        for &(by, bx) in &b_set {
            let ty = by - norm[0].0;
            let tx = bx - norm[0].1;
            let mut ok = true;
            for &(y, x) in &norm {
                if !b_set.contains(&(y + ty, x + tx)) {
                    ok = false;
                    break;
                }
            }
            if ok {
                return true;
            }
        }
    }
    false
}

type Problem = (Vec<Vec<i32>>, Vec<Vec<Option<i32>>>);

fn combinator() -> impl Combinator<Problem> {
    Size::new(Tuple2::new(
        ContextBasedGrid::new(MultiDigit::new(2, 5)),
        ContextBasedGrid::new(Choice::new(vec![
            Box::new(Optionalize::new(HexInt)),
            Box::new(Dict::new(Some(-1), ".")),
            Box::new(Spaces::new(None, 'g')),
        ])),
    ))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = util::infer_shape(&problem.0);
    problem_to_url_with_context(
        combinator(),
        "tridbchoco",
        problem.clone(),
        &Context::sized(h, w),
    )
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["tridbchoco"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        // 9x4: apex=4, rows=4 の正三角形 (16セル)。
        // 手で構成した解のある問題:
        //   灰: (4,0),(5,1),(4,1),(1,3),(2,3),(3,3),(5,2),(6,3)
        let greys = [
            (0, 4),
            (1, 5),
            (1, 4),
            (3, 1),
            (3, 2),
            (3, 3),
            (2, 5),
            (3, 6),
        ];
        let mut color = vec![vec![0; 9]; 4];
        for &(y, x) in &greys {
            color[y][x] = 1;
        }
        let num = vec![vec![None; 9]; 4];
        (color, num)
    }

    #[test]
    fn test_tridbchoco_problem() {
        let problem = problem_for_tests();
        let ans = solve_tridbchoco(&problem.0, &problem.1);
        assert!(ans.is_some());
    }

    #[test]
    fn test_tridbchoco_serializer() {
        let problem = problem_for_tests();
        let url = serialize_problem(&problem).expect("serialize");
        assert!(url.contains("tridbchoco/"));
        let restored = deserialize_problem(&url).expect("deserialize");
        assert_eq!(restored, problem);
    }
}

#[cfg(test)]
mod debug_user_answer {
    use super::*;

    // 行き止まりのある境界線の組み合わせでも正しく合同と判定できること
    // (手入力された解答例: 17x5, 灰8マス, 境界線4本)
    #[test]
    fn test_user_answer_constraint() {
        let greys = [
            (2, 10),
            (3, 5),
            (3, 6),
            (3, 7),
            (3, 8),
            (3, 9),
            (3, 10),
            (3, 11),
        ];
        let mut color = vec![vec![0; 17]; 5];
        for &(y, x) in &greys {
            color[y][x] = 1;
        }
        let num: Vec<Vec<Option<i32>>> = vec![vec![None; 17]; 5];
        let h = 5;
        let w = 17;
        let (apex, rows) = tri_region(h, w);
        assert_eq!((apex, rows), (8, 4));

        let mut constraint = TridbchocoConstraint {
            height: h,
            width: w,
            cell_color: Grid::from_vecs(&color),
            cell_num: Grid::from_vecs(
                &(num.iter()
                    .map(|row| row.iter().map(|&n| n.map(|n| n as usize)).collect::<Vec<_>>())
                    .collect::<Vec<_>>()),
            ),
            decision_stack: vec![],
            border_values: vec![None; 0],
        };
        // border_values の長さ = hor (5*16=80) + ver (上セルが△の数)
        let mut ver_count = 0;
        for y in 0..(h - 1) {
            for x in 0..w {
                if (x + y) % 2 == 0 {
                    ver_count += 1;
                }
            }
        }
        let nvars = h * (w - 1) + ver_count;
        constraint.border_values = vec![Some(false); nvars];
        // ユーザーの解: 壁4本, それ以外は接続
        let walls_hor = [(1, 8), (2, 7), (3, 9)];
        for &(y, x) in &walls_hor {
            let idx = constraint.hor_var(y, x);
            constraint.border_values[idx] = Some(true);
        }
        let walls_ver = [(2, 8)];
        for &(y, x) in &walls_ver {
            assert!((x + y) % 2 == 0);
            let idx = constraint.ver_var(y, x);
            constraint.border_values[idx] = Some(true);
        }

        match constraint.find_inconsistency() {
            Some(reason) => {
                println!("INCONSISTENT: {:?}", reason);
            }
            None => {
                println!("CONSISTENT");
            }
        }
        assert!(constraint.find_inconsistency().is_none());
    }
}
