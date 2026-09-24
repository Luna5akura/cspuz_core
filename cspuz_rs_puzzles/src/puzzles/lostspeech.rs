use crate::util;
use cspuz_rs::serializer::{
    problem_to_url_with_context, url_to_problem, Combinator, Context, DecInt, Dict, MultiDigit,
    Seq, Sequencer, Size, Tuple3,
};
use cspuz_rs::solver::{any, count_true, Solver};

// Lost Speech
//
// Problem = (markers, invalid, pieces)
//   markers[y][x]: -1 = none, 1 = black dot, 2 = hollow dot, 3 = blue dot,
//                  4 = blue-red dot, 5 = triangle, 6 = blue start, 7 = red start,
//                  8 = red dot
//   invalid[y][x]: unusable cell (gray)
//   pieces: bank shapes; even index = blue, odd index = red
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
                        !invalid[cy][cx] && markers[cy][cx] >= 1
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
            solver.add_expr(
                (placed.at(i) & placed.at(j)).imp(ord.at(i).le(ord.at(j) + 1)),
            );
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

pub fn solve_lostspeech(
    markers: &[Vec<i8>],
    invalid: &[Vec<bool>],
    pieces: &[Vec<Vec<bool>>],
) -> Option<(
    Vec<Vec<Option<bool>>>,
    Vec<Vec<Option<bool>>>,
    Vec<Option<bool>>,
    Vec<Option<bool>>,
)> {
    let (h, w) = util::infer_shape(markers);

    let blue_pieces = pieces
        .iter()
        .enumerate()
        .filter_map(|(i, p)| if i % 2 == 0 { Some(p.clone()) } else { None })
        .collect::<Vec<_>>();
    let red_pieces = pieces
        .iter()
        .enumerate()
        .filter_map(|(i, p)| if i % 2 == 1 { Some(p.clone()) } else { None })
        .collect::<Vec<_>>();

    let blue_placements = gen_placements(&blue_pieces, markers, invalid);
    let red_placements = gen_placements(&red_pieces, markers, invalid);

    let mut solver = Solver::new();
    let blue = &solver.bool_var_2d((h, w));
    let red = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(blue);
    solver.add_answer_key_bool(red);

    let blue_placed = solver.bool_var_1d(blue_placements.len());
    let red_placed = solver.bool_var_1d(red_placements.len());
    solver.add_answer_key_bool(&blue_placed);
    solver.add_answer_key_bool(&red_placed);

    // 各マスの被覆: 同色の配置はちょうど1つ
    for y in 0..h {
        for x in 0..w {
            let bcells = blue_placements
                .iter()
                .enumerate()
                .filter_map(|(i, cells)| {
                    if cells.contains(&(y, x)) {
                        Some(blue_placed.at(i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            solver.add_expr(count_true(bcells.clone()).le(1));
            solver.add_expr(blue.at((y, x)).iff(count_true(bcells).eq(1)));

            let rcells = red_placements
                .iter()
                .enumerate()
                .filter_map(|(i, cells)| {
                    if cells.contains(&(y, x)) {
                        Some(red_placed.at(i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            solver.add_expr(count_true(rcells.clone()).le(1));
            solver.add_expr(red.at((y, x)).iff(count_true(rcells).eq(1)));
        }
    }

    // マーカーの条件
    for y in 0..h {
        for x in 0..w {
            let b = blue.at((y, x));
            let r = red.at((y, x));
            match markers[y][x] {
                1 => solver.add_expr(count_true(vec![b.clone(), r.clone()]).eq(1)),
                2 => solver.add_expr(count_true(vec![b.clone(), r.clone()]).le(1)),
                3 => {
                    solver.add_expr(b.clone());
                    solver.add_expr(!r);
                }
                4 => {
                    solver.add_expr(b.clone());
                    solver.add_expr(r.clone());
                }
                6 => solver.add_expr(b),
                7 => solver.add_expr(r),
                8 => {
                    solver.add_expr(r.clone());
                    solver.add_expr(!b);
                }
                _ => (),
            }
        }
    }

    // 連結条件 (同じ色の図形は鎖状に隣接する)
    add_connectivity(&mut solver, &blue_placed, &blue_placements, markers, 6);
    add_connectivity(&mut solver, &red_placed, &red_placements, markers, 7);

    // 別の色の図形に完全に含まれる図形は禁止
    for i in 0..blue_placements.len() {
        let bcells = &blue_placements[i];
        for j in 0..red_placements.len() {
            let rcells = &red_placements[j];
            if bcells.iter().all(|c| rcells.contains(c))
                || rcells.iter().all(|c| bcells.contains(c))
            {
                solver.add_expr(!(blue_placed.at(i) & red_placed.at(j)));
            }
        }
    }

    solver
        .irrefutable_facts()
        .map(|f| (f.get(blue), f.get(red), f.get(&blue_placed), f.get(&red_placed)))
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
        // https://puzz.link/p?lostspeech/3/2/64i7/2/21o/12o
        let url = "https://puzz.link/p?lostspeech/3/2/64i700/2/21o/12o";
        let problem = deserialize_problem(url).unwrap();
        assert_eq!(problem.0.len(), 2);
        assert_eq!(problem.0[0].len(), 3);
        assert_eq!(problem.0[0][0], 6);
        assert_eq!(problem.0[0][1], 4);
        assert_eq!(problem.0[1][2], 7);
        assert_eq!(problem.2.len(), 2);
        assert_eq!(problem.2[0], vec![vec![true, true]]);
        assert_eq!(
            problem.2[1],
            vec![vec![true], vec![true]]
        );

        let reserialized = serialize_problem(&problem).unwrap();
        let problem2 = deserialize_problem(&reserialized).unwrap();
        assert_eq!(problem.0, problem2.0);
        assert_eq!(problem.2, problem2.2);
    }

    #[test]
    fn test_lostspeech_solve() {
        // https://puzz.link/p?lostspeech/3/2/64g22700/2/21o/12o
        let url = "https://puzz.link/p?lostspeech/3/2/642g2700/2/21o/12o";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_lostspeech(&problem.0, &problem.1, &problem.2).unwrap();
        let blue = ans.0;
        let red = ans.1;
        // 青の起点 (0,0) は必ず青
        assert_eq!(blue[0][0], Some(true));
        // 青赤点 (0,1) は必ず青と赤
        assert_eq!(blue[0][1], Some(true));
        assert_eq!(red[0][1], Some(true));
        // 赤の起点 (1,2) は必ず赤
        assert_eq!(red[1][2], Some(true));
    }
}
