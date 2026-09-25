use crate::util;
use cspuz_rs::serializer::{
    problem_to_url_with_context, url_to_problem, Combinator, Context, DecInt, Dict, MultiDigit,
    Seq, Sequencer, Size, Tuple3,
};
use cspuz_rs::solver::{any, count_true, Solver};

// Lost Speech (ツイン盤面)
//
// 2つの同一盤面が左右に並び、マーカー (点・三角・起点・灰色) は共有される。
// 各盤面には青と赤の形状が1つずつ割り当てられ、その形状を起点マスを覆う
// ように1回だけ配置する。点は各盤面の覆われ方を制約する:
//   黒点      : 高々1つの形状
//   黒空心点  : 高々1つの形状
//   青点      : 高々1つの青の形状 (赤なし)
//   青赤点    : 青と赤が1つずつ
//   赤点      : 高々1つの赤の形状 (青なし)
// 青起点はちょうど1つ必要で、青の形状は青起点を覆う。赤起点は任意で、
// ある場合は赤の形状が赤起点を覆う。赤起点が無い場合は赤の形状は置かない。
// 同じ盤面内の青と赤は互いに完全に含まれない。さらに2つの解の間でも、
// どの形状ももう一方の解のどの形状にも完全に含まれてはならない。
pub type Problem = (Vec<Vec<i8>>, Vec<Vec<bool>>, Vec<Vec<Vec<bool>>>);

fn enumerate_piece_transformations(piece: &[Vec<bool>]) -> Vec<Vec<Vec<bool>>> {
    fn rotate_piece_90(piece: &[Vec<bool>]) -> Vec<Vec<bool>> {
        let h = piece.len();
        let w = piece[0].len();
        let mut ret = vec![vec![false; h]; w];
        for i in 0..h {
            for j in 0..w {
                ret[j][h - i - 1] = piece[i][j];
            }
        }
        ret
    }

    fn flip_piece(piece: &[Vec<bool>]) -> Vec<Vec<bool>> {
        let h = piece.len();
        let w = piece[0].len();
        let mut ret = vec![vec![false; w]; h];
        for i in 0..h {
            for j in 0..w {
                ret[i][w - j - 1] = piece[i][j];
            }
        }
        ret
    }

    let mut piece = piece.to_vec();
    let mut ret = vec![];
    for _ in 0..4 {
        ret.push(piece.clone());
        ret.push(flip_piece(&piece));
        piece = rotate_piece_90(&piece);
    }

    let mut ret = ret
        .into_iter()
        .map(|p| {
            // normalize to the top-left
            let mut miny = p.len();
            let mut minx = p[0].len();
            for (y, row) in p.iter().enumerate() {
                for (x, &b) in row.iter().enumerate() {
                    if b {
                        miny = miny.min(y);
                        minx = minx.min(x);
                    }
                }
            }
            let mut norm = vec![vec![false; p[0].len()]; p.len()];
            for y in 0..p.len() {
                for x in 0..p[0].len() {
                    if p[y][x] {
                        norm[y - miny][x - minx] = true;
                    }
                }
            }
            norm
        })
        .collect::<Vec<_>>();
    ret.sort();
    ret.dedup();
    ret
}

pub fn gen_placements(
    pieces: &[Vec<Vec<bool>>],
    markers: &[Vec<i8>],
    invalid: &[Vec<bool>],
) -> Vec<Vec<(usize, usize)>> {
    let h = markers.len();
    let w = markers[0].len();
    let mut ret = vec![];

    for piece in pieces {
        for t in enumerate_piece_transformations(piece) {
            let ph = t.len();
            let pw = t[0].len();
            if ph > h || pw > w {
                continue;
            }
            let cells = t
                .iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.iter()
                        .enumerate()
                        .filter_map(move |(x, &b)| if b { Some((y, x)) } else { None })
                })
                .collect::<Vec<_>>();

            for y in 0..=(h.saturating_sub(ph)) {
                for x in 0..=(w.saturating_sub(pw)) {
                    if cells.iter().all(|&(dy, dx)| {
                        let cy = y + dy;
                        let cx = x + dx;
                        // 形状のすべてのマスは「点」のあるマスでなければならない
                        // (点: 1黒点 2空心点 3青点 4青赤点 8赤点。
                        //  起点マス6/7は除く。三角マーク5は点ではないので覆えない)
                        let m = markers[cy][cx];
                        !invalid[cy][cx] && (m == 1 || m == 2 || m == 3 || m == 4 || m == 6 || m == 7 || m == 8)
                    }) {
                        ret.push(
                            cells
                                .iter()
                                .map(|&(dy, dx)| (y + dy, x + dx))
                                .collect::<Vec<_>>(),
                        );
                    }
                }
            }
        }
    }

    ret
}

pub struct LostSpeechSolveResult {
    pub blue1_cells: Vec<Vec<Option<bool>>>,
    pub red1_cells: Vec<Vec<Option<bool>>>,
    pub blue2_cells: Vec<Vec<Option<bool>>>,
    pub red2_cells: Vec<Vec<Option<bool>>>,
    pub blue1_placements: Vec<Option<bool>>,
    pub red1_placements: Vec<Option<bool>>,
    pub blue2_placements: Vec<Option<bool>>,
    pub red2_placements: Vec<Option<bool>>,
    pub is_unique: bool,
}

// 2つの配置が辺で隣接しているか (両側とも非三角マスで隣接している場合のみ)
fn placements_adjacent(
    cells_a: &[(usize, usize)],
    cells_b: &[(usize, usize)],
    markers: &[Vec<i8>],
) -> bool {
    for &(ay, ax) in cells_a {
        if markers[ay][ax] == 5 {
            continue;
        }
        for &(by, bx) in cells_b {
            if markers[by][bx] == 5 {
                continue;
            }
            if (ay as i32 - by as i32).abs() + (ax as i32 - bx as i32).abs() == 1 {
                return true;
            }
        }
    }
    false
}

// ある色の形状を「起点マスから始まる鎖」に制約する。
// 最初の形状は起点マスを覆い、以降の形状は同じ色の直前の形状と
// 辺で接する (三角マークのマスを通した接続は隣接とみなさない)。
// 起点マスが無い場合はその色の形状は置けない。
fn add_connectivity(
    solver: &mut Solver,
    placed: &cspuz_rs::solver::BoolVarArray1D,
    placements: &[Vec<(usize, usize)>],
    markers: &[Vec<i8>],
    start_marker: i8,
) {
    let n = placements.len();
    if n == 0 {
        return;
    }

    let mut adj = vec![vec![]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if placements_adjacent(&placements[i], &placements[j], markers) {
                adj[i].push(j);
                adj[j].push(i);
            }
        }
    }

    let has_start = markers
        .iter()
        .any(|row| row.iter().any(|&m| m == start_marker));

    let ord = solver.int_var_1d(n, 0, n as i32);

    // 順序はすべて異なる: 形状は「直前の形状の隣」にしか置けないため、
    // アクティブな形状は一筆書きの順序(ハミルトン路)をなす
    for i in 0..n {
        for j in (i + 1)..n {
            solver.add_expr(ord.at(i).ne(ord.at(j)));
        }
    }

    for i in 0..n {
        let is_root = placements[i]
            .iter()
            .any(|&(y, x)| markers[y][x] == start_marker);

        solver.add_expr(placed.at(i).imp(ord.at(i).ge(1)));

        if is_root {
            solver.add_expr(placed.at(i).imp(ord.at(i).eq(1)));
        }

        let mut pred = vec![];
        for &j in &adj[i] {
            // 直前の形状であることの条件: ord_i == ord_j + 1 を満たす
            // 隣接形状が存在すること。
            pred.push(placed.at(j) & ord.at(i).eq(ord.at(j) + 1));
        }
        if !is_root {
            // 有効な配置は、より浅い距離の隣接配置を持たなければならない
            // (隣接が1つも無い場合は any([]) = false となり配置は禁止される)
            solver.add_expr(placed.at(i).imp(any(pred)));
        }
    }

    if !has_start {
        for i in 0..n {
            solver.add_expr(!placed.at(i));
        }
    }
}

// 点のマーカー条件を1つの盤面分だけ課す
fn add_marker_constraints(
    solver: &mut Solver,
    blue: &cspuz_rs::solver::BoolVarArray2D,
    red: &cspuz_rs::solver::BoolVarArray2D,
    markers: &[Vec<i8>],
) {
    let (h, w) = util::infer_shape(markers);
    for y in 0..h {
        for x in 0..w {
            let b = blue.at((y, x));
            let r = red.at((y, x));
            match markers[y][x] {
                1 => solver.add_expr(count_true(vec![b.clone(), r.clone()]).eq(1)),
                2 => solver.add_expr(count_true(vec![b.clone(), r.clone()]).le(1)),
                3 => {
                    // 青点: ちょうど1つの青の図形 (赤なし)
                    solver.add_expr(b);
                    solver.add_expr(!r);
                }
                4 => {
                    // 青赤点: 青と赤が1つずつ
                    solver.add_expr(b);
                    solver.add_expr(r);
                }
                6 => solver.add_expr(b),
                7 => solver.add_expr(r),
                8 => {
                    // 赤点: ちょうど1つの赤の図形 (青なし)
                    solver.add_expr(r.clone());
                    solver.add_expr(!b);
                }
                _ => (),
            }
        }
    }
}

// 2つの形状グループの間で、互いに完全に含まれる形状ペアを禁止する。
fn add_no_containment(
    solver: &mut Solver,
    placed_a: &cspuz_rs::solver::BoolVarArray1D,
    placements_a: &[Vec<(usize, usize)>],
    placed_b: &cspuz_rs::solver::BoolVarArray1D,
    placements_b: &[Vec<(usize, usize)>],
) {
    for (i, cells_a) in placements_a.iter().enumerate() {
        for (j, cells_b) in placements_b.iter().enumerate() {
            let a_contains_b = cells_b.iter().all(|c| cells_a.contains(c));
            let b_contains_a = cells_a.iter().all(|c| cells_b.contains(c));
            if a_contains_b || b_contains_a {
                solver.add_expr(!(placed_a.at(i) & placed_b.at(j)));
            }
        }
    }
}

fn run_solver(
    markers: &[Vec<i8>],
    invalid: &[Vec<bool>],
    pieces: &[Vec<Vec<bool>>],
) -> Option<LostSpeechSolveResult> {
    let (h, w) = util::infer_shape(markers);

    // ピースは4つ: [青1, 赤1, 青2, 赤2] (各盤面に青と赤が1つずつ)
    let piece_defs = (0..4)
        .map(|i| pieces.get(i).cloned())
        .collect::<Vec<_>>();

    let placements = piece_defs
        .iter()
        .map(|p| match p {
            Some(p) => gen_placements(&[p.clone()], markers, invalid),
            None => vec![],
        })
        .collect::<Vec<_>>();

	// 青起点はちょうど1つ必要。赤起点は任意 (無ければ赤の図形は置かない)
	let blue_starts = markers.iter().flatten().filter(|&&m| m == 6).count();
	let red_starts = markers.iter().flatten().filter(|&&m| m == 7).count();
	if blue_starts != 1 || red_starts > 1 {
		return None;
	}

    let mut solver = Solver::new();
    let blue1 = &solver.bool_var_2d((h, w));
    let red1 = &solver.bool_var_2d((h, w));
    let blue2 = &solver.bool_var_2d((h, w));
    let red2 = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(blue1);
    solver.add_answer_key_bool(red1);
    solver.add_answer_key_bool(blue2);
    solver.add_answer_key_bool(red2);

    let p_blue1 = solver.bool_var_1d(placements[0].len());
    let p_red1 = solver.bool_var_1d(placements[1].len());
    let p_blue2 = solver.bool_var_1d(placements[2].len());
    let p_red2 = solver.bool_var_1d(placements[3].len());
    solver.add_answer_key_bool(&p_blue1);
    solver.add_answer_key_bool(&p_red1);
    solver.add_answer_key_bool(&p_blue2);
    solver.add_answer_key_bool(&p_red2);

    // 各色の形状は起点から始まる鎖状に配置する
    add_connectivity(&mut solver, &p_blue1, &placements[0], markers, 6);
    add_connectivity(&mut solver, &p_red1, &placements[1], markers, 7);
    add_connectivity(&mut solver, &p_blue2, &placements[2], markers, 6);
    add_connectivity(&mut solver, &p_red2, &placements[3], markers, 7);

    // 各マスの被覆: 同色の図形は重ならない
    for y in 0..h {
        for x in 0..w {
            let b1 = placements[0]
                .iter()
                .enumerate()
                .filter_map(|(i, cs)| {
                    if cs.contains(&(y, x)) {
                        Some(p_blue1.at(i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            solver.add_expr(count_true(b1.clone()).le(1));
            solver.add_expr(blue1.at((y, x)).iff(count_true(b1).eq(1)));

            let r1 = placements[1]
                .iter()
                .enumerate()
                .filter_map(|(i, cs)| {
                    if cs.contains(&(y, x)) {
                        Some(p_red1.at(i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            solver.add_expr(count_true(r1.clone()).le(1));
            solver.add_expr(red1.at((y, x)).iff(count_true(r1).eq(1)));

            let b2 = placements[2]
                .iter()
                .enumerate()
                .filter_map(|(i, cs)| {
                    if cs.contains(&(y, x)) {
                        Some(p_blue2.at(i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            solver.add_expr(count_true(b2.clone()).le(1));
            solver.add_expr(blue2.at((y, x)).iff(count_true(b2).eq(1)));

            let r2 = placements[3]
                .iter()
                .enumerate()
                .filter_map(|(i, cs)| {
                    if cs.contains(&(y, x)) {
                        Some(p_red2.at(i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            solver.add_expr(count_true(r2.clone()).le(1));
            solver.add_expr(red2.at((y, x)).iff(count_true(r2).eq(1)));
        }
    }

    // 点のマーカー条件 (各盤面ごと)
    add_marker_constraints(&mut solver, blue1, red1, markers);
    add_marker_constraints(&mut solver, blue2, red2, markers);

    // 各盤面内で、青と赤の図形は互いに完全に含まれない
    add_no_containment(&mut solver, &p_blue1, &placements[0], &p_red1, &placements[1]);
    add_no_containment(&mut solver, &p_blue2, &placements[2], &p_red2, &placements[3]);

    // 左右の盤面の間でも、どの図形ももう一方の解のどの図形にも
    // 完全に含まれてはいけない
    add_no_containment(&mut solver, &p_blue1, &placements[0], &p_blue2, &placements[2]);
    add_no_containment(&mut solver, &p_blue1, &placements[0], &p_red2, &placements[3]);
    add_no_containment(&mut solver, &p_red1, &placements[1], &p_blue2, &placements[2]);
    add_no_containment(&mut solver, &p_red1, &placements[1], &p_red2, &placements[3]);

    solver.irrefutable_facts().map(|f| {
        let blue1_cells = f.get(blue1);
        let red1_cells = f.get(red1);
        let blue2_cells = f.get(blue2);
        let red2_cells = f.get(red2);
        let blue1_placements = f.get(&p_blue1);
        let red1_placements = f.get(&p_red1);
        let blue2_placements = f.get(&p_blue2);
        let red2_placements = f.get(&p_red2);
        // 解が一意である ⇔ すべての変数が全解で同じ値を持つ
        let is_unique = blue1_cells.iter().flatten().all(|v| v.is_some())
            && red1_cells.iter().flatten().all(|v| v.is_some())
            && blue2_cells.iter().flatten().all(|v| v.is_some())
            && red2_cells.iter().flatten().all(|v| v.is_some())
            && blue1_placements.iter().all(|v| v.is_some())
            && red1_placements.iter().all(|v| v.is_some())
            && blue2_placements.iter().all(|v| v.is_some())
            && red2_placements.iter().all(|v| v.is_some());
        LostSpeechSolveResult {
            blue1_cells,
            red1_cells,
            blue2_cells,
            red2_cells,
            blue1_placements,
            red1_placements,
            blue2_placements,
            red2_placements,
            is_unique,
        }
    })
}

/// ソルバー表示用: 全解に共通する確定事実のみを返す。
/// 解が一意でない場合でも、どれか1つの解を表示するのではなく、
/// 「どの解でも必ず成り立つ」セル・形状配置のみを返す。
pub fn solve_lostspeech_facts(
    markers: &[Vec<i8>],
    invalid: &[Vec<bool>],
    pieces: &[Vec<Vec<bool>>],
) -> Option<LostSpeechSolveResult> {
    run_solver(markers, invalid, pieces)
}

//---------------------------------------------------------------------------
// URL (de)serialization
//---------------------------------------------------------------------------

struct LostSpeechGrid;

impl Combinator<Vec<Vec<i8>>> for LostSpeechGrid {
    fn serialize(&self, _ctx: &Context, input: &[Vec<Vec<i8>>]) -> Option<(usize, Vec<u8>)> {
        let grid = &input[0];
        let mut ret = vec![];
        let mut run = 0;

        for row in grid {
            for &v in row {
                if v == -1 {
                    run += 1;
                    if run == 20 {
                        ret.push(cspuz_rs::serializer::to_base36(15 + run));
                        run = 0;
                    }
                    continue;
                }
                if run > 0 {
                    ret.push(cspuz_rs::serializer::to_base36(15 + run));
                    run = 0;
                }
                if (0..16).contains(&v) {
                    ret.push(cspuz_rs::serializer::to_base16(v.into()));
                } else {
                    return None;
                }
            }
        }
        if run > 0 {
            ret.push(cspuz_rs::serializer::to_base36(15 + run));
        }

        Some((1, ret))
    }

    fn deserialize(&self, ctx: &Context, input: &[u8]) -> Option<(usize, Vec<Vec<Vec<i8>>>)> {
        let h = ctx.height?;
        let w = ctx.width?;
        let mut ret = vec![vec![-1i8; w]; h];

        let mut c = 0;
        let mut i = 0;
        while i < input.len() && c < h * w {
            let ch = input[i];
            if (b'0'..=b'9').contains(&ch) || (b'a'..=b'f').contains(&ch) {
                ret[c / w][c % w] = cspuz_rs::serializer::from_base16(ch)? as i8;
                c += 1;
                i += 1;
            } else if (b'g'..=b'z').contains(&ch) {
                c += cspuz_rs::serializer::from_base36(ch)? as usize - 15;
                i += 1;
            } else if ch == b'.' {
                c += 1;
                i += 1;
            } else {
                return None;
            }
        }

        if c != h * w {
            return None;
        }
        Some((i, vec![ret]))
    }
}

struct LostSpeechBinary;

impl Combinator<Vec<Vec<bool>>> for LostSpeechBinary {
    fn serialize(&self, ctx: &Context, input: &[Vec<Vec<bool>>]) -> Option<(usize, Vec<u8>)> {
        let h = ctx.height?;
        let w = ctx.width?;
        let grid = &input[0];
        let mut ret = vec![];
        let bits = [16, 8, 4, 2, 1];

        let mut num = 0;
        let mut pass = 0;
        for y in 0..h {
            for x in 0..w {
                if grid[y][x] {
                    pass += bits[num];
                }
                num += 1;
                if num == 5 {
                    ret.push(cspuz_rs::serializer::to_base36(pass));
                    num = 0;
                    pass = 0;
                }
            }
        }
        if num > 0 {
            ret.push(cspuz_rs::serializer::to_base36(pass));
        }
        Some((1, ret))
    }

    fn deserialize(&self, ctx: &Context, input: &[u8]) -> Option<(usize, Vec<Vec<Vec<bool>>>)> {
        let h = ctx.height?;
        let w = ctx.width?;
        let mut ret = vec![vec![false; w]; h];
        let bits = [16, 8, 4, 2, 1];

        let mut c = 0;
        let mut i = 0;
        while i < input.len() && c < h * w {
            let num = cspuz_rs::serializer::from_base36(input[i])?;
            for &b in &bits {
                if c < h * w {
                    ret[c / w][c % w] = (num & b) != 0;
                    c += 1;
                }
            }
            i += 1;
        }
        if c < h * w {
            return None;
        }
        Some((i, vec![ret]))
    }
}

fn square() -> Vec<Vec<bool>> {
    vec![vec![true, true], vec![true, true]]
}

fn domino() -> Vec<Vec<bool>> {
    vec![vec![true, true]]
}

struct LostSpeechPieces;

impl Combinator<Vec<Vec<Vec<bool>>>> for LostSpeechPieces {
    fn serialize(
        &self,
        ctx: &Context,
        input: &[Vec<Vec<Vec<bool>>>],
    ) -> Option<(usize, Vec<u8>)> {
        let data = &input[0];

        if data == &vec![square()] {
            return Some((1, b"//s".to_vec()));
        }
        if data == &vec![domino()] {
            return Some((1, b"//d".to_vec()));
        }
        if data == &vec![domino(), domino()] {
            return Some((1, b"//w".to_vec()));
        }
        if data == &vec![square(), square()] {
            return Some((1, b"//q".to_vec()));
        }
        if data.is_empty() {
            return Some((1, b"//z".to_vec()));
        }

        let mut ret = vec![b'/'];
        let (_, app) = DecInt.serialize(ctx, &[data.len() as i32])?;
        ret.extend(app);

        for piece in data {
            ret.push(b'/');
            let (_, app) = LostSpeechPiece.serialize(ctx, &[piece.clone()])?;
            ret.extend(app);
        }

        Some((1, ret))
    }

    fn deserialize(
        &self,
        ctx: &Context,
        input: &[u8],
    ) -> Option<(usize, Vec<Vec<Vec<Vec<bool>>>>)> {
        let mut sequencer = Sequencer::new(input);

        if sequencer.deserialize(ctx, Dict::new(0, "//s")).is_some() {
            return Some((sequencer.n_read(), vec![vec![square()]]));
        }
        if sequencer.deserialize(ctx, Dict::new(0, "//d")).is_some() {
            return Some((sequencer.n_read(), vec![vec![domino()]]));
        }
        if sequencer.deserialize(ctx, Dict::new(0, "//w")).is_some() {
            return Some((sequencer.n_read(), vec![vec![domino(), domino()]]));
        }
        if sequencer.deserialize(ctx, Dict::new(0, "//q")).is_some() {
            return Some((sequencer.n_read(), vec![vec![square(), square()]]));
        }
        if sequencer.deserialize(ctx, Dict::new(0, "//z")).is_some() {
            return Some((sequencer.n_read(), vec![vec![]]));
        }

        sequencer.deserialize(ctx, Dict::new(0, "/"))?;

        let n_pieces = sequencer.deserialize(ctx, DecInt)?;
        assert_eq!(n_pieces.len(), 1);
        let n_pieces = n_pieces[0] as usize;

        let mut pieces = vec![];
        for _ in 0..n_pieces {
            sequencer.deserialize(ctx, Dict::new(0, "/"))?;
            let piece = sequencer.deserialize_one_elem(ctx, LostSpeechPiece)?;
            pieces.push(piece);
        }

        Some((sequencer.n_read(), vec![pieces]))
    }
}

struct LostSpeechPiece;

impl Combinator<Vec<Vec<bool>>> for LostSpeechPiece {
    fn serialize(&self, _ctx: &Context, input: &[Vec<Vec<bool>>]) -> Option<(usize, Vec<u8>)> {
        let data = &input[0];
        let height = data.len();
        let width = data[0].len();

        if !((1..=35).contains(&height) && (1..=35).contains(&width)) {
            return None;
        }

        let mut ret = vec![];
        let (_, app) = MultiDigit::new(36, 1).serialize(_ctx, &[width as i32])?;
        ret.extend(app);
        let (_, app) = MultiDigit::new(36, 1).serialize(_ctx, &[height as i32])?;
        ret.extend(app);

        let mut seq = vec![];
        for row in data {
            for &b in row {
                seq.push(if b { 1 } else { 0 });
            }
        }
        while seq.last() == Some(&0) {
            seq.pop();
        }
        let (_, app) = Seq::new(MultiDigit::new(2, 5), seq.len())
            .serialize(&Context::sized(height, width), &[seq])?;
        ret.extend(app);

        Some((1, ret))
    }

    fn deserialize(&self, _ctx: &Context, input: &[u8]) -> Option<(usize, Vec<Vec<Vec<bool>>>)> {
        let mut sequencer = Sequencer::new(input);

        let width = sequencer.deserialize(_ctx, MultiDigit::new(36, 1))?;
        assert_eq!(width.len(), 1);
        let width = width[0] as usize;

        let height = sequencer.deserialize(_ctx, MultiDigit::new(36, 1))?;
        assert_eq!(height.len(), 1);
        let height = height[0] as usize;

        let mut ret = vec![vec![false; width]; height];
        let mut pos = 0;
        while pos < height * width {
            if let Some(subseq) = sequencer.deserialize(_ctx, MultiDigit::new(2, 5)) {
                for i in 0..subseq.len() {
                    if pos >= height * width {
                        break;
                    }
                    ret[pos / width][pos % width] = subseq[i] == 1;
                    pos += 1;
                }
            } else {
                break;
            }
        }

        Some((sequencer.n_read(), vec![ret]))
    }
}

fn combinator() -> impl Combinator<Problem> {
    Size::new(Tuple3::new(
        LostSpeechGrid,
        LostSpeechBinary,
        LostSpeechPieces,
    ))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let height = problem.0.len();
    let width = problem.0[0].len();
    problem_to_url_with_context(
        combinator(),
        "lostspeech",
        problem.clone(),
        &Context::sized(height, width),
    )
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["lostspeech"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lostspeech_url() {
        // ツイン盤面: マーカー + 4つのピース [青1, 赤1, 青2, 赤2]
        let url = "https://puzz.link/p?lostspeech/2/3/6117h1g/4/12o/12o/21o/21o";
        let problem = deserialize_problem(url).unwrap();
        assert_eq!(problem.0.len(), 3);
        assert_eq!(problem.0[0].len(), 2);
        assert_eq!(problem.0[0][0], 6);
        assert_eq!(problem.0[0][1], 1);
        assert_eq!(problem.0[1][0], 1);
        assert_eq!(problem.0[1][1], 7);
        assert_eq!(problem.1[2][0], true);
        assert_eq!(problem.1[2][1], true);
        assert_eq!(problem.2.len(), 4);
        assert_eq!(problem.2[0], vec![vec![true], vec![true]]);
        assert_eq!(problem.2[1], vec![vec![true], vec![true]]);
        assert_eq!(problem.2[2], vec![vec![true, true]]);
        assert_eq!(problem.2[3], vec![vec![true, true]]);

        let reserialized = serialize_problem(&problem).unwrap();
        let problem2 = deserialize_problem(&reserialized).unwrap();
        assert_eq!(problem.0, problem2.0);
        assert_eq!(problem.1, problem2.1);
        assert_eq!(problem.2, problem2.2);
    }

    #[test]
    fn test_lostspeech_unique() {
        // 4x4盤: 青起点(0,0), 赤起点(3,3), その他は空心点または無印。
        // 盤面1: 2x2正方形×2 → 解は一意。
        // 盤面2: Tテトリミノ×2 → 解は一意。
        // 2つの解の形状は互いに完全に含まれないため、共同の解も一意。
        let url = "https://puzz.link/p?lostspeech/4/4/622g222gh22g2270000/4/22u/22u/32t0/23eg";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech_facts(&problem.0, &problem.1, &problem.2).unwrap();

        assert!(ans.is_unique);
        assert!(ans.blue1_cells.iter().flatten().all(|v| v.is_some()));
        assert!(ans.red1_cells.iter().flatten().all(|v| v.is_some()));
        assert!(ans.blue2_cells.iter().flatten().all(|v| v.is_some()));
        assert!(ans.red2_cells.iter().flatten().all(|v| v.is_some()));

        // 盤面1: 2x2正方形
        for &(y, x) in &[(0, 0), (0, 1), (1, 0), (1, 1)] {
            assert_eq!(ans.blue1_cells[y][x], Some(true));
        }
        for &(y, x) in &[(2, 2), (2, 3), (3, 2), (3, 3)] {
            assert_eq!(ans.red1_cells[y][x], Some(true));
        }
        // 盤面2: Tテトリミノ
        for &(y, x) in &[(0, 0), (0, 1), (0, 2), (1, 1)] {
            assert_eq!(ans.blue2_cells[y][x], Some(true));
        }
        for &(y, x) in &[(2, 2), (3, 1), (3, 2), (3, 3)] {
            assert_eq!(ans.red2_cells[y][x], Some(true));
        }
    }

    #[test]
    fn test_lostspeech_non_unique() {
        // 4x4: 盤面1は2x2正方形×2 (一意)、盤面2は青=3連トロミノ (縦/横の2通り) +
        // 赤=Tテトリミノ。盤面2の青に2つの置き方があるため非一意。
        // (跨盤包含制約はすべて満たす: 正方形とトロミノ・Tは互いに包含しない)
        let url = "https://puzz.link/p?lostspeech/4/4/6222222g2g22g2270000/4/22u/22u/13s/23eg";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech_facts(&problem.0, &problem.1, &problem.2).unwrap();

        assert!(!ans.is_unique);
        // 盤面1は確定
        for &(y, x) in &[(0usize, 0usize), (0, 1), (1, 0), (1, 1)] {
            assert_eq!(ans.blue1_cells[y][x], Some(true));
        }
        for &(y, x) in &[(2usize, 2usize), (2, 3), (3, 2), (3, 3)] {
            assert_eq!(ans.red1_cells[y][x], Some(true));
        }
        // 盤面2の青は起点(0,0)だけが確定
        assert_eq!(ans.blue2_cells[0][0], Some(true));
        assert_eq!(ans.blue2_cells[1][0], None);
        assert_eq!(ans.blue2_cells[0][1], None);
        // 盤面2の赤Tは確定
        for &(y, x) in &[(2usize, 2usize), (3, 1), (3, 2), (3, 3)] {
            assert_eq!(ans.red2_cells[y][x], Some(true));
        }
    }

    #[test]
    fn test_lostspeech_within_board_containment_rejected() {
        // 3x3: 青起点(0,0), 赤起点(1,1), 空心点(0,1),(1,0), 青赤点(0,2),(1,2)。
        // 青の鎖: {(0,0),(0,1)} と {(0,2),(1,2)}、赤の鎖: {(1,0),(1,1)} と {(0,2),(1,2)}。
        // 起点を含まない2つ目の形状どうしが同一セル集合のため、
        // 盤面内の包含制約で解なしになる。
        let url = "https://puzz.link/p?lostspeech/3/3/624274i00/4/12o/12o/12o/12o";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech_facts(&problem.0, &problem.1, &problem.2);
        assert!(ans.is_none(), "contained shapes on the same board must be rejected");
    }

    #[test]
    fn test_lostspeech_start_cells_count_for_containment() {
        // 3x3: 青起点(0,0), 赤起点(1,1), 空心点(0,1),(0,2),(1,0),(2,0)。
        // 赤=単セル(1,1)が青の2x2正方形に完全に含まれる (起点豁免なし) → 解なし。
        let url = "https://puzz.link/p?lostspeech/3/3/62227g2h00/4/22u/11g/13s/11g";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech_facts(&problem.0, &problem.1, &problem.2);
        assert!(ans.is_none(), "contained shapes must be rejected (no start exemption)");
    }

    #[test]
    fn test_lostspeech_example() {
        // ユーザー提供の例题 (8x8)。青起点(4,1), 青点(5,1)。赤起点なし。
        // 盤面1: L形の鎖、盤面2: ドミノの鎖。
        // 起点豁免の無い包含判定では、青点(5,1)を両盤面の青形状が覆うため
        // 必ず包含が発生し、解なしになる。
        let url = "https://puzz.link/p?lostspeech/8/8/y122j1111i611g11g23g2222w0000000000000/4/22e/22u/12o/22u";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech_facts(&problem.0, &problem.1, &problem.2);
        assert!(ans.is_none(), "nesting forced by the shared start and blue dot");
    }

    #[test]
    fn test_lostspeech_example2() {
        // ユーザー提供の例题その2 (8x8)。青起点(3,1), 青点(4,1)。赤起点なし。
        // 起点豁免の無い包含判定では、青点(4,1)を両盤面の青形状が覆うため
        // 必ず包含が発生し、解なしになる。
        let url = "https://puzz.link/p?lostspeech/8/8/q12k1111i611g11g23g2222zk0000000000000/4/12o/22u/22e/22u";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech_facts(&problem.0, &problem.1, &problem.2);
        assert!(ans.is_none(), "nesting forced by the shared start and blue dot");
    }

    #[test]
    fn test_lostspeech_example3() {
        // ユーザー提供の例题その3 (8x8)。青起点(3,2), 三角(3,1)。赤起点なし。
        // 青点が無いため、起点豁免の無い包含判定の下で一意に解ける。
        let url = "https://puzz.link/p?lostspeech/8/8/q122j1111i561g11j2222zk0000000000000/4/12o/22u/22e/22u";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech_facts(&problem.0, &problem.1, &problem.2).unwrap();

        assert!(ans.is_unique);
        // 盤面1: ドミノの鎖
        for &(y, x) in &[
            (1usize, 3usize),
            (1, 4),
            (2, 2),
            (2, 3),
            (2, 4),
            (2, 5),
            (3, 2),
            (3, 3),
            (3, 5),
            (3, 6),
            (4, 5),
            (4, 6),
        ] {
            assert_eq!(ans.blue1_cells[y][x], Some(true));
        }
        // 盤面2: L形の鎖
        for &(y, x) in &[
            (1usize, 3usize),
            (1, 4),
            (1, 5),
            (2, 2),
            (2, 3),
            (2, 4),
            (2, 5),
            (3, 2),
            (3, 3),
            (3, 5),
            (3, 6),
            (4, 3),
        ] {
            assert_eq!(ans.blue2_cells[y][x], Some(true));
        }
        // 赤は両盤面とも置かれない
        assert!(ans.red1_cells.iter().flatten().all(|v| v == &Some(false)));
        assert!(ans.red2_cells.iter().flatten().all(|v| v == &Some(false)));
    }
}
