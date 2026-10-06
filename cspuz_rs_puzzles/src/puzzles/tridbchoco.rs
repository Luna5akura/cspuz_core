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

    let mut constraint = TridbchocoConstraint {
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

    fn find_inconsistency_inner(&mut self) -> Option<Vec<(usize, bool)>> {
        let (h, w) = (self.height, self.width);

        let (unit_id, units) = self.compute_components(false, false);
        let (punit_id, punits) = self.compute_components(false, true);
        let (block_id, blocks) = self.compute_components(true, false);

        // 潜在ブロック(色を無視し、未確定の辺は接続とみなす)は、
        // 最終的に1つのブロックにまとまるので、両方の色を含む必要がある
        let (pblock_id, pblocks) = self.compute_components(true, true);
        for pb in &pblocks {
            let mut has = [false, false];
            for &(y, x) in pb {
                has[self.cell_color[(y, x)] as usize] = true;
            }
            if !has[0] || !has[1] {
                return Some(self.reason_for_region(pb, &pblock_id));
            }
        }

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
                    let mut ret = self.reason_for_connected_edges(block, &block_id);
                    ret.extend(self.reason_for_region(&punits[pid], &punit_id));
                    ret.sort();
                    ret.dedup();
                    return Some(ret);
                }
                size_by_color[c] += 1;
                if let Some(n) = self.cell_num[(y, x)] {
                    has_num[c] = true;
                    match num {
                        Some(m) if m != n => {
                            return Some(self.reason_for_connected_edges(block, &block_id));
                        }
                        _ => num = Some(n),
                    }
                }
            }

            // 潜在ユニットが相手の色の大きさに到達できない
            if punit_by_color[0] != !0 && punits[punit_by_color[0]].len() < size_by_color[1] {
                let mut ret = self.reason_for_region(&punits[punit_by_color[0]], &punit_id);
                ret.extend(self.reason_for_connected_edges(block, &block_id));
                ret.sort();
                ret.dedup();
                return Some(ret);
            }
            if punit_by_color[1] != !0 && punits[punit_by_color[1]].len() < size_by_color[0] {
                let mut ret = self.reason_for_region(&punits[punit_by_color[1]], &punit_id);
                ret.extend(self.reason_for_connected_edges(block, &block_id));
                ret.sort();
                ret.dedup();
                return Some(ret);
            }

            if let Some(n) = num {
                // 確定済みのカタマリが数字より大きい
                if n < size_by_color[0] || n < size_by_color[1] {
                    return Some(self.reason_for_connected_edges(block, &block_id));
                }
                // 潜在ユニットが数字に到達できない
                for c in 0..2 {
                    if punit_by_color[c] != !0 && n > punits[punit_by_color[c]].len() {
                        if has_num[c] || size_by_color[1 - c] > 0 {
                            let mut ret =
                                self.reason_for_region(&punits[punit_by_color[c]], &punit_id);
                            ret.extend(self.reason_for_connected_edges(block, &block_id));
                            ret.sort();
                            ret.dedup();
                            return Some(ret);
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
                        let mut ret = self.reason_for_path(y, x, y2, x2, &block_id);
                        ret.push((self.border_idx(y, x, y2, x2), true));
                        return Some(ret);
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
                let mut ret = self.reason_for_connected_edges(unit, &unit_id);
                ret.extend(self.reason_for_region(&punits[pid], &punit_id));
                for &other_pid in &punit_adj[pid] {
                    let (oy, ox) = punits[other_pid][0];
                    if self.cell_color[(oy, ox)] == color {
                        continue;
                    }
                    ret.extend(self.reason_for_region(&punits[other_pid], &punit_id));
                }
                ret.sort();
                ret.dedup();
                return Some(ret);
            }
        }

        None
    }

    // セル集合 cells の周囲の壁 (group における境界の辺) を理由として返す
    fn reason_for_region(
        &self,
        cells: &[(usize, usize)],
        group: &Grid<usize>,
    ) -> Vec<(usize, bool)> {
        let (h, w) = (self.height, self.width);
        let id = group[(cells[0].0, cells[0].1)];
        let mut ret = vec![];
        for &(y, x) in cells {
            for &(y2, x2) in &self.tri_neighbors(y, x) {
                if !tri_in_region(h, w, y2, x2) {
                    continue;
                }
                if group[(y2, x2)] != id
                    && self.border_state(y, x, y2, x2) == BorderState::Wall
                {
                    ret.push((self.border_idx(y, x, y2, x2), true));
                }
            }
        }
        ret.sort();
        ret.dedup();
        ret
    }

    fn border_idx(&self, y: usize, x: usize, y2: usize, x2: usize) -> usize {
        if y == y2 {
            self.hor_var(y, x.min(x2))
        } else {
            self.ver_var(y.min(y2), x)
        }
    }

    // 領域 cells の内部の、接続と確定している辺を理由として返す
    fn reason_for_connected_edges(
        &self,
        cells: &[(usize, usize)],
        group: &Grid<usize>,
    ) -> Vec<(usize, bool)> {
        let id = group[(cells[0].0, cells[0].1)];
        let mut ret = vec![];
        for &(y, x) in cells {
            for &(y2, x2) in &self.tri_neighbors(y, x) {
                if group[(y2, x2)] == id
                    && self.border_state(y, x, y2, x2) == BorderState::Connected
                {
                    ret.push((self.border_idx(y, x, y2, x2), false));
                }
            }
        }
        ret.sort();
        ret.dedup();
        ret
    }

    // (y,x) と (y2,x2) を同じブロック内の非壁の辺で結ぶ経路を返す
    fn reason_for_path(
        &self,
        y: usize,
        x: usize,
        y2: usize,
        x2: usize,
        block_id: &Grid<usize>,
    ) -> Vec<(usize, bool)> {
        let (h, w) = (self.height, self.width);
        let mut prev: Grid<Option<(usize, usize)>> = Grid::new(h, w, None);
        prev[(y, x)] = Some((y, x));
        let mut stack = vec![(y, x)];
        while let Some((cy, cx)) = stack.pop() {
            for &(ny, nx) in &self.tri_neighbors(cy, cx) {
                if prev[(ny, nx)].is_some() {
                    continue;
                }
                if block_id[(ny, nx)] != block_id[(y, x)] {
                    continue;
                }
                if self.border_state(cy, cx, ny, nx) != BorderState::Connected {
                    continue;
                }
                prev[(ny, nx)] = Some((cy, cx));
                if (ny, nx) == (y2, x2) {
                    break;
                }
                stack.push((ny, nx));
            }
        }
        let mut ret = vec![];
        let mut cur = (y2, x2);
        while cur != (y, x) {
            let p = prev[(cur.0, cur.1)].unwrap();
            ret.push((self.border_idx(cur.0, cur.1, p.0, p.1), false));
            cur = p;
        }
        ret
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
    // 矛盾判定は重いので、伝播キューが空になるまで遅延させる
    fn lazy_propagation(&self) -> bool {
        true
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
        let mut ret = self.find_inconsistency_inner();
        if let Some(ref mut reason) = ret {
            // 理由には、直近に確定したリテラル(現在の決定レベル)が
            // 含まれている必要がある
            if let Some(&last) = self.decision_stack.last() {
                if !reason.iter().any(|&(i, _)| i == last) {
                    reason.push((last, self.border_values[last].unwrap()));
                }
            }
        }
        #[cfg(test)]
        if let Some(ref reason) = ret {
            for &(i, v) in reason {
                assert_eq!(
                    self.border_values[i],
                    Some(v),
                    "bad reason lit: border {} = {} (actual {:?})",
                    i,
                    v,
                    self.border_values[i]
                );
            }
        }
        ret
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

// 形状を正規化したキー (点をソートした文字列) を作る
fn tri_shape_key(pts: &[(i32, i32)]) -> String {
    let mut pts = pts.to_vec();
    pts.sort();
    pts.iter()
        .map(|&(y, x)| format!("{},{}", y, x))
        .collect::<Vec<_>>()
        .join("/")
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

    if a.len() == b.len() {
        // 大きさが同じ場合: 正規化キーの一致だけで判定できる
        let mut b_keys = std::collections::HashSet::new();
        for i in 0..12 {
            let mut t: Vec<(i32, i32)> = b_set.iter().map(|&p| tri_apply_map(i, p)).collect();
            let mut min_y = i32::MAX;
            let mut min_x = i32::MAX;
            for &(y, x) in &t {
                min_y = min_y.min(y);
                min_x = min_x.min(x);
            }
            for p in &mut t {
                p.0 -= min_y;
                p.1 -= min_x;
            }
            b_keys.insert(tri_shape_key(&t));
        }
        for i in 0..12 {
            let mut t: Vec<(i32, i32)> = pts.iter().map(|&p| tri_apply_map(i, p)).collect();
            let mut min_y = i32::MAX;
            let mut min_x = i32::MAX;
            for &(y, x) in &t {
                min_y = min_y.min(y);
                min_x = min_x.min(x);
            }
            for p in &mut t {
                p.0 -= min_y;
                p.1 -= min_x;
            }
            if b_keys.contains(&tri_shape_key(&t)) {
                return true;
            }
        }
        return false;
    }

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
mod user_answer_valid_tests {
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

#[cfg(test)]
mod user_answer_invalid_tests {
    use super::*;

    #[test]
    fn test_user_answer2_constraint() {
        // ユーザーの解答例2: 17x5, 灰8マス, 壁6本 (うち1本は盤外)
        let greys = [
            (1, 8),
            (2, 7),
            (2, 10),
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
        // ユーザーの解: 壁
        let walls_hor = [(1, 7), (1, 8), (2, 6), (2, 7), (3, 9)];
        for &(y, x) in &walls_hor {
            let idx = constraint.hor_var(y, x);
            constraint.border_values[idx] = Some(true);
        }
        // (17,8) の壁は (8,3)-(8,4) 間で、▽の下辺なので実在しない

        match constraint.find_inconsistency() {
            Some(_reason) => {
                println!("INCONSISTENT (rejected)");
                assert!(true);
            }
            None => {
                println!("CONSISTENT (accepted)");
                assert!(false, "user answer2 should be rejected");
            }
        }
    }
}

#[cfg(test)]
mod instance_solutions_tests {
    use super::*;

    // 17x5 の例題は複数解を持つことを確認する
    #[test]
    fn test_enum_solutions() {
        let greys = [
            (1, 8),
            (2, 7),
            (2, 10),
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

        let mut solver = Solver::new();
        let borders = TriBorders::new(&mut solver, 5, 17);
        solver.add_answer_key_bool(&borders.vars);

        let cell_num = Grid::from_vecs(
            &(num.iter()
                .map(|row| row.iter().map(|&n| n.map(|n| n as usize)).collect::<Vec<_>>())
                .collect::<Vec<_>>()),
        );
        let mut constraint = TridbchocoConstraint {
            height: 5,
            width: 17,
            cell_color: Grid::from_vecs(&color),
            cell_num,
            decision_stack: vec![],
            border_values: vec![None; borders.vars.len()],
        };
        solver.add_custom_constraint(Box::new(constraint), borders.vars.clone());

        let mut count = 0;
        let mut first = None;
        for ans in solver.answer_iter().take(200) {
            count += 1;
            if first.is_none() {
                first = Some(borders.extract(&ans));
            }
        }
        // 複数解あることを確認 (200個以上)
        assert!(count >= 200);
    }
}

#[cfg(test)]
mod perf_tests {
    use super::*;
    use std::time::Instant;

    // 17x8 の例題 (64セル) の求解時間と呼び出し回数を計測する
    #[test]
    fn test_slow_instance_perf() {
        let greys = [
            (2, 6), (2, 7), (2, 9), (2, 10),
            (3, 5), (3, 7), (3, 10), (3, 11),
            (4, 4), (4, 5), (4, 6), (4, 11), (4, 12),
            (5, 3), (5, 4), (5, 5), (5, 9), (5, 12), (5, 13),
            (6, 3), (6, 4), (6, 5), (6, 6), (6, 8), (6, 9), (6, 11), (6, 12),
            (7, 4), (7, 5), (7, 6), (7, 10), (7, 11),
        ];
        let mut color = vec![vec![0; 17]; 8];
        for &(y, x) in &greys {
            color[y][x] = 1;
        }
        let num: Vec<Vec<Option<i32>>> = vec![vec![None; 17]; 8];

        let mut solver = Solver::new();
        let borders = TriBorders::new(&mut solver, 8, 17);
        solver.add_answer_key_bool(&borders.vars);
        let cell_num = Grid::from_vecs(
            &(num.iter()
                .map(|row| row.iter().map(|&n| n.map(|n| n as usize)).collect::<Vec<_>>())
                .collect::<Vec<_>>()),
        );
        let mut constraint = TridbchocoConstraint {
            height: 8,
            width: 17,
            cell_color: Grid::from_vecs(&color),
            cell_num,
            decision_stack: vec![],
            border_values: vec![None; borders.vars.len()],
        };
        solver.add_custom_constraint(Box::new(constraint), borders.vars.clone());

        let t0 = Instant::now();
        let facts = solver.irrefutable_facts();
        let dt = t0.elapsed().as_secs_f64();
        println!("perf: solve took {:.2}s, calls=?", dt);
        // constraint は solver にムーブされたので参照できないため、
        // カウンタは外に出す
        let _ = facts;
    }
}
