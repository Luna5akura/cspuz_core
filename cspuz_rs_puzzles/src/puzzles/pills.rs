use crate::util;
use cspuz_rs::serializer::strip_prefix;
use cspuz_rs::solver::{count_true, sum, Solver};

/// A Pills problem consists of a grid of given dot counts together with the
/// row and column clues.  Pill values are not stored explicitly: they are
/// always 1..N where N is the (unique) number of pills, which is derived from
/// the triangular sum of the clues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PillsProblem {
    pub dots: Vec<Vec<i32>>,
    pub row_clues: Vec<Option<i32>>,
    pub col_clues: Vec<Option<i32>>,
}

/// Returns N such that N * (N + 1) / 2 == sum.
fn triangular_root(sum: i32) -> Option<i32> {
    if sum < 0 {
        return None;
    }
    let root = (1.0 + 8.0 * sum as f64).sqrt() as i64;
    let mid = (root - 1) / 2;
    for n in (mid - 2).max(0)..=(mid + 2) {
        if n * (n + 1) / 2 == sum as i64 {
            return Some(n as i32);
        }
    }
    None
}

struct Placement {
    cells: [(usize, usize); 3],
    dot_sum: i32,
    row_dots: Vec<i32>,
    col_dots: Vec<i32>,
}

pub fn solve_pills(problem: &PillsProblem) -> Option<Vec<Vec<Option<i32>>>> {
    let (h, w) = util::infer_shape(&problem.dots);
    if h == 0 || w == 0 || problem.row_clues.len() != h || problem.col_clues.len() != w {
        return None;
    }

    let row_clues: Vec<i32> = problem.row_clues.iter().copied().collect::<Option<_>>()?;
    let col_clues: Vec<i32> = problem.col_clues.iter().copied().collect::<Option<_>>()?;

    let row_sum: i32 = row_clues.iter().sum();
    let col_sum: i32 = col_clues.iter().sum();
    if row_sum != col_sum {
        return None;
    }
    let num_pills = triangular_root(row_sum)?;

    // Enumerate all straight 1x3/3x1 placements whose dot sum is a legal pill
    // value.  Only placements covering at least one dot can be used.
    let mut placements: Vec<Placement> = vec![];
    for y in 0..h {
        for x in 0..w.saturating_sub(2) {
            let cells = [(y, x), (y, x + 1), (y, x + 2)];
            let dot_sum: i32 = cells
                .iter()
                .map(|&(yy, xx)| problem.dots[yy][xx].max(0))
                .sum();
            if (1..=num_pills).contains(&dot_sum) {
                let mut row_dots = vec![0; h];
                let mut col_dots = vec![0; w];
                for &(yy, xx) in &cells {
                    row_dots[yy] += problem.dots[yy][xx].max(0);
                    col_dots[xx] += problem.dots[yy][xx].max(0);
                }
                placements.push(Placement {
                    cells,
                    dot_sum,
                    row_dots,
                    col_dots,
                });
            }
        }
    }
    for y in 0..h.saturating_sub(2) {
        for x in 0..w {
            let cells = [(y, x), (y + 1, x), (y + 2, x)];
            let dot_sum: i32 = cells
                .iter()
                .map(|&(yy, xx)| problem.dots[yy][xx].max(0))
                .sum();
            if (1..=num_pills).contains(&dot_sum) {
                let mut row_dots = vec![0; h];
                let mut col_dots = vec![0; w];
                for &(yy, xx) in &cells {
                    row_dots[yy] += problem.dots[yy][xx].max(0);
                    col_dots[xx] += problem.dots[yy][xx].max(0);
                }
                placements.push(Placement {
                    cells,
                    dot_sum,
                    row_dots,
                    col_dots,
                });
            }
        }
    }

    let mut solver = Solver::new();
    let used = &solver.bool_var_1d(placements.len());
    solver.add_answer_key_bool(used);

    // Every cell is covered by at most one pill.
    let mut covering = vec![vec![]; h * w];
    for (i, placement) in placements.iter().enumerate() {
        for &(y, x) in &placement.cells {
            covering[y * w + x].push(i);
        }
    }
    for y in 0..h {
        for x in 0..w {
            let terms: Vec<_> = covering[y * w + x].iter().map(|&i| used.at(i)).collect();
            if !terms.is_empty() {
                solver.add_expr(count_true(terms).le(1));
            }
        }
    }

    // Exactly one pill of every value.
    solver.add_expr(count_true(used).eq(num_pills));
    for value in 1..=num_pills {
        let mut terms = vec![];
        for (i, placement) in placements.iter().enumerate() {
            if placement.dot_sum == value {
                terms.push(used.at(i));
            }
        }
        solver.add_expr(count_true(terms).eq(1));
    }

    // Row and column clues count the dots that are inside pills.
    for y in 0..h {
        let mut terms = vec![];
        for (i, placement) in placements.iter().enumerate() {
            if placement.row_dots[y] != 0 {
                terms.push(used.at(i).ite(placement.row_dots[y], 0));
            }
        }
        solver.add_expr(sum(terms).eq(row_clues[y]));
    }
    for x in 0..w {
        let mut terms = vec![];
        for (i, placement) in placements.iter().enumerate() {
            if placement.col_dots[x] != 0 {
                terms.push(used.at(i).ite(placement.col_dots[x], 0));
            }
        }
        solver.add_expr(sum(terms).eq(col_clues[x]));
    }

    let facts = solver.irrefutable_facts()?;
    let used_vals = facts.get(used);

    let mut answer = vec![vec![None; w]; h];
    for y in 0..h {
        for x in 0..w {
            let mut value = Some(0);
            for &i in &covering[y * w + x] {
                match used_vals.get(i).copied().flatten() {
                    Some(true) => {
                        value = Some(placements[i].dot_sum);
                    }
                    Some(false) => {}
                    None => {
                        value = None;
                    }
                }
            }
            answer[y][x] = value;
        }
    }

    Some(answer)
}

fn hex_digit(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'f' => Some((c - b'a' + 10) as i32),
        b'A'..=b'F' => Some((c - b'A' + 10) as i32),
        _ => None,
    }
}

fn parse_hex(input: &[u8], pos: usize, len: usize) -> Option<i32> {
    let mut value = 0;
    for i in 0..len {
        value = value * 16 + hex_digit(*input.get(pos + i)?)?;
    }
    Some(value)
}

/// Decode one pzpr number16 record.  Mirrors `Encode.readNumber16`.
fn read_number16(input: &[u8], pos: usize) -> Option<(i32, usize)> {
    let c = *input.get(pos)?;
    match c {
        b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => Some((hex_digit(c)?, 1)),
        b'.' => Some((-2, 1)),
        b'-' => Some((parse_hex(input, pos + 1, 2)?, 3)),
        b'+' => Some((parse_hex(input, pos + 1, 3)?, 4)),
        b'=' => Some((parse_hex(input, pos + 1, 3)? + 4096, 4)),
        b'%' | b'@' => Some((parse_hex(input, pos + 1, 3)? + 8192, 4)),
        b'*' => Some((parse_hex(input, pos + 1, 4)? + 12240, 5)),
        b'$' => Some((parse_hex(input, pos + 1, 5)? + 77776, 6)),
        _ => None,
    }
}

/// Decode a pzpr number16 stream of `length` values starting at `pos`.
/// Missing values default to -1, matching the pzpr decoder.
fn decode_number16(input: &[u8], mut pos: usize, length: usize) -> Option<(Vec<i32>, usize)> {
    let mut ret = vec![-1; length];
    let mut index = 0usize;
    while index < length && pos < input.len() {
        let ca = input[pos];
        if let Some((value, consumed)) = read_number16(input, pos) {
            ret[index] = value;
            index += 1;
            pos += consumed;
        } else if (b'g'..=b'z').contains(&ca) {
            index += (ca - b'g') as usize + 1;
            pos += 1;
        } else {
            pos += 1;
        }
    }
    Some((ret, pos))
}

fn write_number16(value: i32, out: &mut Vec<u8>) {
    fn push_hex(value: i32, len: usize, out: &mut Vec<u8>) {
        for i in (0..len).rev() {
            out.push(b"0123456789abcdef"[((value >> (4 * i)) & 15) as usize]);
        }
    }
    match value {
        -2 => out.push(b'.'),
        0..=15 => push_hex(value, 1, out),
        16..=255 => {
            out.push(b'-');
            push_hex(value, 2, out);
        }
        256..=4095 => {
            out.push(b'+');
            push_hex(value, 3, out);
        }
        4096..=8191 => {
            out.push(b'=');
            push_hex(value - 4096, 3, out);
        }
        8192..=12239 => {
            out.push(b'@');
            push_hex(value - 8192, 3, out);
        }
        12240..=77775 => {
            out.push(b'*');
            push_hex(value - 12240, 4, out);
        }
        77776..=1126351 => {
            out.push(b'$');
            push_hex(value - 77776, 5, out);
        }
        _ => {}
    }
}

fn encode_number16(values: &[i32], out: &mut Vec<u8>) {
    let mut run = 0usize;
    let flush_run = |run: &mut usize, out: &mut Vec<u8>| {
        if *run > 0 {
            out.push(b"ghijklmnopqrstuvwxyz"[*run - 1]);
            *run = 0;
        }
    };
    for &value in values {
        if value < 0 && value != -2 {
            run += 1;
            if run == 20 {
                flush_run(&mut run, out);
            }
            continue;
        }
        flush_run(&mut run, out);
        write_number16(value, out);
    }
    flush_run(&mut run, out);
}

pub fn deserialize_problem(url: &str) -> Option<PillsProblem> {
    let serialized = strip_prefix(url)?;
    let mut parts = serialized.split('/');
    if parts.next()? != "pills" {
        return None;
    }
    let cols: usize = parts.next()?.parse().ok()?;
    let rows: usize = parts.next()?.parse().ok()?;
    let body = parts.next().unwrap_or("").as_bytes();
    if cols == 0 || rows == 0 {
        return None;
    }

    let (dots_flat, pos) = decode_number16(body, 0, cols.checked_mul(rows)?)?;
    let (clues, _) = decode_number16(body, pos, cols + rows)?;

    let mut dots = vec![vec![0; cols]; rows];
    for y in 0..rows {
        for x in 0..cols {
            dots[y][x] = dots_flat[y * cols + x].max(0);
        }
    }
    let col_clues = clues[0..cols]
        .iter()
        .map(|&v| if v >= 0 { Some(v) } else { None })
        .collect();
    let row_clues = clues[cols..cols + rows]
        .iter()
        .map(|&v| if v >= 0 { Some(v) } else { None })
        .collect();

    Some(PillsProblem {
        dots,
        row_clues,
        col_clues,
    })
}

pub fn serialize_problem(problem: &PillsProblem) -> Option<String> {
    let (h, w) = util::infer_shape(&problem.dots);
    if h == 0 || w == 0 || problem.row_clues.len() != h || problem.col_clues.len() != w {
        return None;
    }

    let mut body = vec![];
    let mut dots_flat = vec![];
    for y in 0..h {
        for x in 0..w {
            dots_flat.push(problem.dots[y][x].max(0));
        }
    }
    encode_number16(&dots_flat, &mut body);

    let mut clues = vec![];
    for &clue in &problem.col_clues {
        clues.push(clue.unwrap_or(-1));
    }
    for &clue in &problem.row_clues {
        clues.push(clue.unwrap_or(-1));
    }
    encode_number16(&clues, &mut body);

    Some(format!(
        "https://puzz.link/p?pills/{w}/{h}/{}",
        String::from_utf8(body).ok()?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_20a() -> PillsProblem {
        // WPF GP 2015 Round 7, puzzle 20 (10x10).
        PillsProblem {
            dots: vec![
                vec![4, 4, 4, 2, 2, 2, 2, 4, 4, 4],
                vec![4, 2, 4, 1, 1, 1, 1, 4, 2, 4],
                vec![4, 4, 4, 1, 0, 0, 1, 4, 4, 4],
                vec![2, 1, 1, 1, 0, 0, 1, 1, 1, 2],
                vec![2, 1, 0, 0, 0, 0, 0, 0, 1, 2],
                vec![2, 1, 0, 0, 0, 0, 0, 0, 1, 2],
                vec![2, 1, 1, 1, 0, 0, 1, 1, 1, 2],
                vec![4, 4, 4, 1, 0, 0, 1, 4, 4, 4],
                vec![4, 2, 4, 1, 1, 1, 1, 4, 2, 4],
                vec![4, 4, 4, 2, 2, 2, 2, 4, 4, 4],
            ],
            row_clues: vec![
                Some(8),
                Some(5),
                Some(4),
                Some(1),
                Some(3),
                Some(1),
                Some(4),
                Some(14),
                Some(13),
                Some(2),
            ],
            col_clues: vec![
                Some(4),
                Some(8),
                Some(17),
                Some(1),
                Some(1),
                Some(2),
                Some(7),
                Some(5),
                Some(8),
                Some(2),
            ],
        }
    }

    #[test]
    fn test_pills_triangular_root() {
        assert_eq!(triangular_root(55), Some(10));
        assert_eq!(triangular_root(78), Some(12));
        assert_eq!(triangular_root(54), None);
        assert_eq!(triangular_root(0), Some(0));
    }

    #[test]
    fn test_pills_solves_gp_2015_round7_20() {
        let problem = problem_20a();
        let answer = solve_pills(&problem).expect("GP puzzle 20a must have an answer");

        let expected = crate::util::tests::to_option_2d([
            [0, 0, 0, 0, 0, 8, 8, 8, 0, 0],
            [0, 0, 9, 0, 1, 0, 0, 0, 0, 0],
            [0, 0, 9, 0, 1, 0, 0, 0, 0, 0],
            [0, 0, 9, 0, 1, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 3, 3, 3],
            [0, 6, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 6, 0, 0, 0, 2, 2, 2, 7, 0],
            [0, 6, 5, 5, 5, 0, 4, 0, 7, 0],
            [10, 10, 10, 0, 0, 0, 4, 0, 7, 0],
            [0, 0, 0, 0, 0, 0, 4, 0, 0, 0],
        ]);

        assert_eq!(answer, expected);
    }

    #[test]
    fn test_pills_serializer_roundtrip() {
        let problem = problem_20a();
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
        assert!(url.starts_with("https://puzz.link/p?pills/10/10/"));
    }

    #[test]
    fn test_pills_decodes_pzprjs_url() {
        // URL produced by the pzpr editor for the same puzzle; verifies the
        // Rust decoder against the real pzpr number16 encoding.
        let url = "https://puzz.link/p?pills/10/10/444222244442411114244441001444211100111221000000122100000012211100111244410014444241111424444222244448-1111275828541314ed2";
        assert_eq!(deserialize_problem(url), Some(problem_20a()));
    }

    #[test]
    fn test_pills_empty_grid_and_invalid_clues() {
        // No dots and all-zero clues means there are no pills at all.
        let empty = PillsProblem {
            dots: vec![vec![0; 3]; 3],
            row_clues: vec![Some(0); 3],
            col_clues: vec![Some(0); 3],
        };
        assert_eq!(solve_pills(&empty), Some(vec![vec![Some(0); 3]; 3]));

        // A clue sum that is not a triangular number cannot describe a set
        // of pills with values 1..N.
        let invalid = PillsProblem {
            dots: vec![vec![0; 3]; 3],
            row_clues: vec![Some(1), Some(0), Some(0)],
            col_clues: vec![Some(1), Some(0), Some(0)],
        };
        assert_eq!(solve_pills(&invalid), None);

        // Missing clues are not supported by the solver.
        let missing = PillsProblem {
            dots: vec![vec![0; 3]; 3],
            row_clues: vec![Some(0), None, Some(0)],
            col_clues: vec![Some(0); 3],
        };
        assert_eq!(solve_pills(&missing), None);
    }
}
