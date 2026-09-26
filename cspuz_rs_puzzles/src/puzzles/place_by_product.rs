// Place by Product solver
//
// Rules (https://puzz.link/rules.html?placebyproduct):
// - Place every shape from the bank into the grid exactly once. Shapes can be
//   rotated or mirrored, and no other shapes may appear in the grid.
// - Two shapes cannot touch each other, not even diagonally.
// - The shapes divide each row and column into groups of consecutive white
//   cells. Numbers outside the grid indicate the product of the sizes of these
//   groups in the corresponding row or column. A 0 clue means the row or
//   column is completely filled with shapes.
// - Some cells may be given as pre-filled parts of shapes.

use crate::util;
use cspuz_rs::solver::{any, count_true, sum, Solver};

// Problem = (given cells, row clues, column clues, pieces)
//   given: pre-filled cells (must be covered by a placed piece)
//   row_clues[y] / col_clues[x]: product clue, -1 = no clue, 0 = fully shaded
pub type Problem = (
    Vec<Vec<bool>>,
    Vec<i32>,
    Vec<i32>,
    Vec<Vec<Vec<bool>>>,
);

// ---------- URL parsing ----------

fn from_hex(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'f' => Some((c - b'a' + 10) as i32),
        b'A'..=b'F' => Some((c - b'A' + 10) as i32),
        _ => None,
    }
}

// pzpr Number16 encoding (shared with battleships): 0-15 as one hex digit,
// 16-255 as "-XX", 256-4095 as "+XXX", "." = dot (-2), letters g-z skip runs
// of empty cells (-1).
fn parse_number16(data: &str, n: usize) -> Option<(Vec<i32>, usize)> {
    let bytes = data.as_bytes();
    let mut vals = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let ca = bytes[i];
        if let Some(v) = from_hex(ca) {
            vals.push(v);
            i += 1;
        } else if ca == b'-' {
            let hi = from_hex(*bytes.get(i + 1)?)?;
            let lo = from_hex(*bytes.get(i + 2)?)?;
            vals.push(hi * 16 + lo);
            i += 3;
        } else if ca == b'+' {
            let a = from_hex(*bytes.get(i + 1)?)?;
            let b = from_hex(*bytes.get(i + 2)?)?;
            let c = from_hex(*bytes.get(i + 3)?)?;
            vals.push(a * 256 + b * 16 + c);
            i += 4;
        } else if ca == b'=' {
            let a = from_hex(*bytes.get(i + 1)?)?;
            let b = from_hex(*bytes.get(i + 2)?)?;
            let c = from_hex(*bytes.get(i + 3)?)?;
            vals.push(a * 256 + b * 16 + c + 4096);
            i += 4;
        } else if ca == b'@' || ca == b'%' {
            let a = from_hex(*bytes.get(i + 1)?)?;
            let b = from_hex(*bytes.get(i + 2)?)?;
            let c = from_hex(*bytes.get(i + 3)?)?;
            vals.push(a * 256 + b * 16 + c + 8192);
            i += 4;
        } else if ca == b'*' {
            let mut v = 0;
            for k in 1..=4 {
                v = v * 16 + from_hex(*bytes.get(i + k)?)?;
            }
            vals.push(v + 12240);
            i += 5;
        } else if ca == b'$' {
            let mut v = 0;
            for k in 1..=5 {
                v = v * 16 + from_hex(*bytes.get(i + k)?)?;
            }
            vals.push(v + 77776);
            i += 6;
        } else if ca == b'.' {
            vals.push(-2);
            i += 1;
        } else if (b'g'..=b'z').contains(&ca) {
            let run = (ca - b'a' + 10) - 15;
            for _ in 0..run {
                vals.push(-1);
            }
            i += 1;
        } else {
            return None;
        }
        if vals.len() == n {
            return Some((vals, i));
        }
    }
    if vals.len() == n {
        Some((vals, i))
    } else {
        None
    }
}

// Bank piece ("wh+base32 bits" or the raw "w:0/1 string" form)
fn deserialize_piece(str: &str) -> Option<Vec<Vec<bool>>> {
    if str.contains(':') {
        let mut it = str.split(':');
        let w: usize = it.next()?.parse().ok()?;
        let s = it.next()?;
        let h = s.len() / w;
        if w == 0 || h == 0 {
            return None;
        }
        let mut ret = vec![vec![false; w]; h];
        for (i, ch) in s.bytes().enumerate() {
            if i < w * h {
                ret[i / w][i % w] = ch == b'1';
            }
        }
        Some(ret)
    } else {
        let bytes = str.as_bytes();
        if bytes.len() < 3 {
            return None;
        }
        let w = (bytes[0] as char).to_digit(36)? as usize;
        let h = (bytes[1] as char).to_digit(36)? as usize;
        if w == 0 || h == 0 {
            return None;
        }
        let mut bits = vec![];
        for &c in &bytes[2..] {
            let v = (c as char).to_digit(32)? as u32;
            for k in (0..5).rev() {
                bits.push(((v >> k) & 1) == 1);
            }
        }
        let mut ret = vec![vec![false; w]; h];
        for i in 0..(w * h).min(bits.len()) {
            ret[i / w][i % w] = bits[i];
        }
        Some(ret)
    }
}

// Bank presets (shortkey -> piece strings), matching the pzprjs bank presets
fn preset_pieces(shortkey: &str) -> Option<Vec<&'static str>> {
    match shortkey {
        // tetrominoes
        "t" => Some(vec!["14u", "23bg", "22u", "23f", "23eg"]),
        // double tetrominoes
        "d" => Some(vec![
            "14u", "14u", "23bg", "23bg", "22u", "22u", "23f", "23f", "23eg", "23eg",
        ]),
        // pentominoes
        "p" => Some(vec![
            "337k", "15v", "24as", "24bo", "23fg", "337i", "23rg", "334u", "335s", "33bk",
            "24bk", "337o",
        ]),
        // zero (empty bank)
        "z" => Some(vec![]),
        _ => None,
    }
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    // URL: .../placebyproduct/w/h/<number16(excell)><base27(cells)>/<bank>
    //   bank: "/<count>/<piece>/..." or preset "//<shortkey>"
    let body = url.split("placebyproduct/").nth(1)?;
    let mut segs = body.split('/');
    let w: usize = segs.next()?.parse().ok()?;
    let h: usize = segs.next()?.parse().ok()?;
    let data = segs.next()?;

    let mut pieces = vec![];
    let bank_head = segs.next()?;
    if bank_head.is_empty() {
        // preset form ("//<shortkey>")
        let shortkey = segs.next()?;
        for p in preset_pieces(shortkey)? {
            pieces.push(deserialize_piece(p)?);
        }
    } else {
        let count: usize = bank_head.parse().ok()?;
        for _ in 0..count {
            pieces.push(deserialize_piece(segs.next()?)?);
        }
    }

    // excell (top = column clues w, left = row clues h) + cells (base27)
    let mut col_clues = vec![-1; w];
    let mut row_clues = vec![-1; h];
    let cell_part = if data.is_empty() {
        // shorthand URLs such as ".../4/4///t" carry no clue payload
        ""
    } else {
        let (excell_vals, used) = parse_number16(data, w + h)?;
        for x in 0..w {
            col_clues[x] = excell_vals[x];
        }
        for y in 0..h {
            row_clues[y] = excell_vals[w + y];
        }
        &data[used..]
    };

    // cells: base-27 digits, 3 cells per character, row-major order;
    // value 2 = given (pre-filled) cell
    let mut given = vec![vec![false; w]; h];
    if !cell_part.is_empty() {
        let bytes = cell_part.as_bytes();
        let n_chars = (h * w + 2) / 3;
        if bytes.len() < n_chars {
            return None;
        }
        for i in 0..(h * w) {
            let digit = (bytes[i / 3] as char).to_digit(27)?;
            let tri = [9u32, 3u32, 1u32];
            let val = (digit / tri[i % 3]) % 3;
            if val == 2 {
                given[i / w][i % w] = true;
            }
        }
    }

    Some((given, row_clues, col_clues, pieces))
}

// ---------- Solver ----------

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

fn enumerate_piece_transformations(piece: &[Vec<bool>]) -> Vec<Vec<Vec<bool>>> {
    let mut piece = piece.to_vec();
    let mut ret = vec![];
    for _ in 0..4 {
        ret.push(piece.clone());
        ret.push(flip_piece(&piece));
        piece = rotate_piece_90(&piece);
    }
    ret
}

// Model the product constraint for one row/column.
//
// For clue c >= 1: white cells form runs; each run has a length v (a divisor
// of c, at most the line length). The encoding ties v == 0 to shaded cells,
// forces each run to span exactly v cells (v_{i+t} == v_i while t < v_i, and
// v_{i+v_i} == 0 right after the run), and enforces the product of run
// lengths to equal c via per-prime-factor exponent sums.
fn add_product_line_constraint(
    solver: &mut Solver,
    line: &[(usize, usize)],
    covered: &cspuz_rs::solver::BoolVarArray2D,
    clue: i32,
) {
    let n = line.len();
    if clue == 0 {
        // completely filled with shapes
        for &(y, x) in line {
            solver.add_expr(covered.at((y, x)));
        }
        return;
    }
    let c = clue as usize;

    // candidate run lengths = divisors of c not exceeding the line length
    let divisors: Vec<i32> = (1..=c)
        .filter(|&d| c % d == 0 && d <= n)
        .map(|d| d as i32)
        .collect();
    let max_d = *divisors.last().unwrap();

    // v[i] = 0 (shaded) or the length of the white run containing cell i
    let v = solver.int_var_1d(n, 0, max_d);
    for i in 0..n {
        let (y, x) = line[i];
        let vi = v.at(i);
        for d in 1..=max_d {
            if c as i32 % d != 0 {
                solver.add_expr(vi.ne(d));
            }
        }
        solver.add_expr(covered.at((y, x)).iff(vi.eq(0)));
    }

    // at least one white cell
    solver.add_expr(any(line.iter().map(|&(y, x)| !covered.at((y, x)))));

    for i in 0..n {
        let vi = v.at(i);
        let run_start = if i == 0 {
            vi.ne(0)
        } else {
            vi.ne(0) & v.at(i - 1).eq(0)
        };
        for t in 1..(n - i) {
            let tj = t as i32;
            // cells strictly inside the run carry the same run length
            solver.add_expr((run_start.clone() & vi.gt(tj)).imp(v.at(i + t).eq(vi.clone())));
            // the first cell after the run must be shaded
            solver.add_expr((run_start.clone() & vi.eq(tj)).imp(v.at(i + t).eq(0)));
        }
    }

    // product of run lengths == c: for each prime factor p^e of c, the sum of
    // the p-exponents of all run lengths must equal e
    let mut rem = c as i32;
    let mut p = 2;
    while p * p <= rem {
        if rem % p == 0 {
            let mut e = 0;
            while rem % p == 0 {
                rem /= p;
                e += 1;
            }
            add_prime_exponent_sum(solver, &line, &v, &divisors, p, e);
        }
        p += 1;
    }
    if rem > 1 {
        add_prime_exponent_sum(solver, &line, &v, &divisors, rem, 1);
    }
}

fn add_prime_exponent_sum(
    solver: &mut Solver,
    line: &[(usize, usize)],
    v: &cspuz_rs::solver::IntVarArray1D,
    divisors: &[i32],
    p: i32,
    e: i32,
) {
    let n = line.len();
    let exponent_of = |d: i32| -> i32 {
        let mut d = d;
        let mut ret = 0;
        while d % p == 0 {
            d /= p;
            ret += 1;
        }
        ret
    };

    let mut terms = vec![];
    for i in 0..n {
        let vi = v.at(i);
        let run_start = if i == 0 {
            vi.ne(0)
        } else {
            vi.ne(0) & v.at(i - 1).eq(0)
        };
        // exponent of p in v[i] (0 when v[i] == 0)
        let mut e_p = vi.eq(divisors[0]).ite(exponent_of(divisors[0]), 0);
        for &d in &divisors[1..] {
            e_p = vi.eq(d).ite(exponent_of(d), e_p);
        }
        terms.push(run_start.ite(e_p, 0));
    }
    solver.add_expr(sum(terms).eq(e));
}

pub fn solve_place_by_product(problem: &Problem) -> Option<Vec<Vec<Option<bool>>>> {
    let (given, row_clues, col_clues, pieces) = problem;
    let h = given.len();
    let w = given[0].len();

    // Each distinct transformation of each bank piece gets unique ids for its
    // cells (statue_park-style encoding): a cell's state is the id of the
    // piece cell covering it, or 0 when the cell is white.
    let mut id = 1;
    let mut piece_transformations_ids_all = vec![];
    let mut leader_ids_all = vec![];

    for piece in pieces {
        let mut transformations = enumerate_piece_transformations(piece);
        transformations.sort();
        transformations.dedup();

        let mut piece_transformations_ids = vec![];
        let mut leader_ids = vec![];
        for t in transformations {
            let (ph, pw) = util::infer_shape(&t);
            let mut ids = vec![];
            let mut ld = None;
            for y in 0..ph {
                let mut row = vec![];
                for x in 0..pw {
                    if t[y][x] {
                        if ld.is_none() {
                            ld = Some(id);
                        }
                        row.push(Some(id));
                        id += 1;
                    } else {
                        row.push(None);
                    }
                }
                ids.push(row);
            }
            piece_transformations_ids.push(ids);

            assert!(ld.is_some());
            leader_ids.push(ld.unwrap());
        }
        piece_transformations_ids_all.push(piece_transformations_ids);
        leader_ids_all.push(leader_ids);
    }

    let mut solver = Solver::new();
    let is_block = &solver.bool_var_2d((h, w));

    // Each cell's state domain only contains the ids whose piece placement can
    // legally anchor at this cell (plus 0 = white). This keeps the SAT
    // encoding compact and lets invalid placements be constant-folded away.
    let mut cell_states = vec![];
    for y in 0..h {
        let mut row = vec![];
        for x in 0..w {
            let mut cands = vec![0];
            for i in 0..piece_transformations_ids_all.len() {
                for j in 0..piece_transformations_ids_all[i].len() {
                    let piece_transformations_ids = &piece_transformations_ids_all[i][j];
                    let (ph, pw) = util::infer_shape(piece_transformations_ids);
                    for py in 0..ph {
                        for px in 0..pw {
                            if let Some(pid) = piece_transformations_ids[py][px] {
                                if y >= py && x >= px && y + ph - py <= h && x + pw - px <= w {
                                    cands.push(pid);
                                }
                            }
                        }
                    }
                }
            }
            cands.sort();
            cands.dedup();
            row.push(solver.int_var_from_domain(cands));
        }
        cell_states.push(row);
    }
    let cell_state = &cspuz_rs::solver::IntVarArray2D::new(
        (h, w),
        cell_states.into_iter().flat_map(|row| row.into_iter()),
    );

    solver.add_answer_key_bool(is_block);
    solver.add_expr(is_block.iff(cell_state.ne(0)));

    for y in 0..h {
        for x in 0..w {
            for i in 0..piece_transformations_ids_all.len() {
                for j in 0..piece_transformations_ids_all[i].len() {
                    let piece_transformations_ids = &piece_transformations_ids_all[i][j];
                    let (ph, pw) = util::infer_shape(piece_transformations_ids);

                    for py in 0..ph {
                        for px in 0..pw {
                            if let Some(pid) = piece_transformations_ids[py][px] {
                                if !(y >= py && x >= px && y + ph - py <= h && x + pw - px <= w) {
                                    continue;
                                }

                                for (dy, dx) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                                    let pyi = py as i32;
                                    let pxi = px as i32;

                                    let py2 = pyi + dy;
                                    let px2 = pxi + dx;
                                    let y2 = y as i32 + dy;
                                    let x2 = x as i32 + dx;

                                    let id2 = if 0 <= py2
                                        && py2 < ph as i32
                                        && 0 <= px2
                                        && px2 < pw as i32
                                    {
                                        piece_transformations_ids[py2 as usize][px2 as usize]
                                    } else {
                                        None
                                    };

                                    if let Some(id2) = id2 {
                                        solver.add_expr(cell_state.at((y, x)).eq(pid).imp(
                                            cell_state.at((y2 as usize, x2 as usize)).eq(id2),
                                        ));
                                    } else if 0 <= y2 && y2 < h as i32 && 0 <= x2 && x2 < w as i32
                                    {
                                        solver.add_expr(cell_state.at((y, x)).eq(pid).imp(
                                            cell_state.at((y2 as usize, x2 as usize)).eq(0),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // every bank piece is used exactly once
    for i in 0..pieces.len() {
        let mut inds = vec![];
        for y in 0..h {
            for x in 0..w {
                let mut ind = vec![];
                for &j in leader_ids_all[i].iter() {
                    ind.push(cell_state.at((y, x)).eq(j));
                }
                inds.push(any(ind));
            }
        }
        solver.add_expr(count_true(inds).eq(1));
    }

    // piece index covering each cell (0 = white): the cell_state ids differ
    // between the cells of a piece, so piece membership is tracked separately
    let piece_id = &solver.int_var_2d((h, w), 0, pieces.len() as i32);
    for y in 0..h {
        for x in 0..w {
            solver.add_expr(piece_id.at((y, x)).eq(0).iff(cell_state.at((y, x)).eq(0)));
            for p in 0..pieces.len() {
                let mut ind = vec![];
                for j in 0..piece_transformations_ids_all[p].len() {
                    let tids = &piece_transformations_ids_all[p][j];
                    let (ph, pw) = util::infer_shape(tids);
                    for py in 0..ph {
                        for px in 0..pw {
                            if let Some(pid) = tids[py][px] {
                                ind.push(cell_state.at((y, x)).eq(pid));
                            }
                        }
                    }
                }
                solver.add_expr(piece_id.at((y, x)).eq(p as i32 + 1).iff(any(ind)));
            }
        }
    }

    // pieces cannot touch diagonally (orthogonal touching is already
    // impossible because of the neighbor linking): two diagonally adjacent
    // cells must belong to the same piece or one of them must be white
    for y in 0..h {
        for x in 0..w {
            for &(dy, dx) in &[(1i32, 1i32), (1, -1)] {
                let ny = y as i32 + dy;
                let nx = x as i32 + dx;
                if ny < 0 || nx < 0 || ny >= h as i32 || nx >= w as i32 {
                    continue;
                }
                let (ny, nx) = (ny as usize, nx as usize);
                solver.add_expr(
                    (piece_id.at((y, x)).eq(0) | piece_id.at((ny, nx)).eq(0))
                        | piece_id.at((y, x)).eq(piece_id.at((ny, nx))),
                );
            }
        }
    }

    // given cells are parts of pieces
    for y in 0..h {
        for x in 0..w {
            if given[y][x] {
                solver.add_expr(is_block.at((y, x)));
            }
        }
    }

    // product clues
    for y in 0..h {
        if row_clues[y] >= 0 {
            let line: Vec<(usize, usize)> = (0..w).map(|x| (y, x)).collect();
            add_product_line_constraint(&mut solver, &line, is_block, row_clues[y]);
        }
    }
    for x in 0..w {
        if col_clues[x] >= 0 {
            let line: Vec<(usize, usize)> = (0..h).map(|y| (y, x)).collect();
            add_product_line_constraint(&mut solver, &line, is_block, col_clues[x]);
        }
    }

    solver.irrefutable_facts().map(|f| f.get(is_block))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_option_bool_2d(v: Vec<Vec<i32>>) -> Vec<Vec<Option<bool>>> {
        v.into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|c| match c {
                        1 => Some(true),
                        0 => Some(false),
                        _ => None,
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn test_deserialize_basic() {
        // https://puzz.link/p?placebyproduct/2/2/021100/1/12o
        let problem = deserialize_problem("https://puzz.link/p?placebyproduct/2/2/021100/1/12o")
            .unwrap();
        assert_eq!(problem.1, vec![1, 1]); // row clues
        assert_eq!(problem.2, vec![0, 2]); // column clues
        assert_eq!(problem.0, vec![vec![false; 2]; 2]); // no given cells
        assert_eq!(problem.3, vec![vec![vec![true], vec![true]]]); // domino
    }

    #[test]
    fn test_deserialize_example() {
        // https://puzz.link/p?placebyproduct/4/4/22401133000000/2/22u/14u
        let problem = deserialize_problem(
            "https://puzz.link/p?placebyproduct/4/4/22401133000000/2/22u/14u",
        )
        .unwrap();
        assert_eq!(problem.1, vec![1, 1, 3, 3]);
        assert_eq!(problem.2, vec![2, 2, 4, 0]);
        assert_eq!(
            problem.3,
            vec![
                vec![vec![true, true], vec![true, true]],
                vec![vec![true], vec![true], vec![true], vec![true]],
            ]
        );
    }

    #[test]
    fn test_deserialize_preset_and_given() {
        // tetromino preset + a given cell at (0, 0)
        let problem =
            deserialize_problem("https://puzz.link/p?placebyproduct/2/2/0211i0/1/12o").unwrap();
        assert_eq!(problem.1, vec![1, 1]);
        assert_eq!(problem.2, vec![0, 2]);
        assert_eq!(problem.0, vec![vec![true, false], vec![false, false]]);
        assert_eq!(problem.3, vec![vec![vec![true], vec![true]]]);

        let problem =
            deserialize_problem("https://puzz.link/p?placebyproduct/4/4/n000000//t").unwrap();
        assert_eq!(problem.1, vec![-1; 4]);
        assert_eq!(problem.2, vec![-1; 4]);
        assert_eq!(problem.3.len(), 5); // tetrominoes
    }

    #[test]
    fn test_solve_small() {
        // https://puzz.link/p?placebyproduct/2/2/021100/1/12o
        let problem =
            deserialize_problem("https://puzz.link/p?placebyproduct/2/2/021100/1/12o").unwrap();
        let ans = solve_place_by_product(&problem).unwrap();
        let expected = to_option_bool_2d(vec![vec![1, 0], vec![1, 0]]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_solve_example() {
        // https://puzz.link/p?placebyproduct/4/4/22401133000000/2/22u/14u
        let problem = deserialize_problem(
            "https://puzz.link/p?placebyproduct/4/4/22401133000000/2/22u/14u",
        )
        .unwrap();
        let ans = solve_place_by_product(&problem).unwrap();
        let expected = to_option_bool_2d(vec![
            vec![1, 1, 0, 1],
            vec![1, 1, 0, 1],
            vec![0, 0, 0, 1],
            vec![0, 0, 0, 1],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_solve_given_cell() {
        // https://puzz.link/p?placebyproduct/3/1/10012/1/12o
        // Given cell at (0, 2) forces the domino to cover columns 1-2.
        let problem =
            deserialize_problem("https://puzz.link/p?placebyproduct/3/1/10012/1/12o").unwrap();
        let ans = solve_place_by_product(&problem).unwrap();
        let expected = to_option_bool_2d(vec![vec![0, 1, 1]]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_solve_all_shaded() {
        // https://puzz.link/p?placebyproduct/1/1/000/1/11g
        // 1x1 board, clue 0 in the row and column, one monomino in the bank.
        let problem =
            deserialize_problem("https://puzz.link/p?placebyproduct/1/1/000/1/11g").unwrap();
        let ans = solve_place_by_product(&problem).unwrap();
        let expected = to_option_bool_2d(vec![vec![1]]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_solve_impossible() {
        // The monomino must be placed, but clue 1 forces a white cell.
        let problem =
            deserialize_problem("https://puzz.link/p?placebyproduct/1/1/110/1/11g").unwrap();
        assert!(solve_place_by_product(&problem).is_none());
    }

    #[test]
    fn test_solve_tetrominoes_8x6() {
        // Generated valid puzzle (tetromino preset); the answer is verified
        // against the pzprjs answer checker.
        let problem = deserialize_problem(
            "https://puzz.link/p?placebyproduct/8/6/322345211754380000000000000000//t",
        )
        .unwrap();
        let ans = solve_place_by_product(&problem).unwrap();
        let expected = to_option_bool_2d(vec![
            vec![1, 1, 1, 1, 0, 1, 1, 1],
            vec![0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 1, 1, 0, 0, 0, 0, 0],
            vec![0, 1, 0, 0, 0, 0, 1, 1],
            vec![0, 0, 0, 1, 1, 0, 1, 1],
            vec![0, 0, 1, 1, 0, 0, 0, 0],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_solve_tetrominoes_7x7() {
        // Generated valid puzzle (tetromino preset); the answer is verified
        // against the pzprjs answer checker.
        let problem = deserialize_problem(
            "https://puzz.link/p?placebyproduct/7/7/2226428371472100000000000000000//t",
        )
        .unwrap();
        let ans = solve_place_by_product(&problem).unwrap();
        let expected = to_option_bool_2d(vec![
            vec![1, 1, 1, 1, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0],
            vec![1, 1, 1, 0, 1, 1, 1],
            vec![0, 0, 1, 0, 0, 1, 0],
            vec![0, 0, 0, 0, 0, 0, 0],
            vec![1, 1, 0, 0, 1, 1, 0],
            vec![0, 1, 1, 0, 1, 1, 0],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_solve_double_tetrominoes() {
        // Generated valid puzzle (double tetromino preset: two of each
        // tetromino); the answer is verified against the pzprjs answer checker.
        let problem = deserialize_problem(
            "https://puzz.link/p?placebyproduct/10/10/9482c82c841c-10448-12c210000000000000000000000000000000000//d",
        )
        .unwrap();
        let ans = solve_place_by_product(&problem).unwrap();
        let expected = to_option_bool_2d(vec![
            vec![0, 1, 1, 1, 0, 1, 1, 1, 0, 1],
            vec![0, 0, 1, 0, 0, 0, 1, 0, 0, 1],
            vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
            vec![1, 1, 0, 1, 1, 0, 0, 0, 0, 1],
            vec![0, 1, 0, 1, 0, 0, 1, 1, 0, 0],
            vec![0, 1, 0, 0, 0, 0, 1, 1, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0],
            vec![1, 0, 0, 1, 0, 0, 1, 0, 0, 0],
            vec![1, 1, 0, 1, 0, 0, 1, 0, 1, 1],
            vec![0, 1, 0, 1, 0, 1, 1, 0, 1, 1],
        ]);
        assert_eq!(ans, expected);
    }
}

