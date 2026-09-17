//! Solver and PuzzLink serializer for Sky Neighbors.
//!
//! Sky Neighbors is the 9x9 Neighbors puzzle with one ring of nine cells on
//! each side of the board.  The ring cells are part of the answer: a ring
//! value is the number of visible buildings when the corresponding row or
//! column is viewed from that side.

use cspuz_rs::serializer::{problem_to_url_with_context, strip_prefix, Combinator, Context, Size};
use cspuz_rs::solver::{all, count_true, Solver};

pub const SKY_NEIGHBOR_SIZE: usize = 9;
pub const OUTER_SIZE: usize = 4 * SKY_NEIGHBOR_SIZE;

/// Inner givens, outer givens, inner outlined flags, outer outlined flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub inner_givens: Vec<Vec<Option<i32>>>,
    pub outer_givens: Vec<Option<i32>>,
    pub inner_outlined: Vec<Vec<bool>>,
    pub outer_outlined: Vec<bool>,
}

fn is_9x9<T>(grid: &[Vec<T>]) -> bool {
    grid.len() == SKY_NEIGHBOR_SIZE && grid.iter().all(|row| row.len() == SKY_NEIGHBOR_SIZE)
}

fn valid_num(n: Option<i32>) -> bool {
    n.is_none_or(|n| (1..=3).contains(&n))
}

fn valid_problem(problem: &Problem) -> bool {
    is_9x9(&problem.inner_givens)
        && problem.outer_givens.len() == OUTER_SIZE
        && is_9x9(&problem.inner_outlined)
        && problem.outer_outlined.len() == OUTER_SIZE
        && problem
            .inner_givens
            .iter()
            .flatten()
            .copied()
            .all(valid_num)
        && problem.outer_givens.iter().copied().all(valid_num)
}

/// Return all graph neighbours of an answer position.  The first component
/// identifies an inner cell, and the second component is its index.
fn neighbours_of(y: usize, x: usize) -> Vec<(bool, usize)> {
    let mut ret = Vec::with_capacity(4);
    if y > 0 {
        ret.push((true, (y - 1) * SKY_NEIGHBOR_SIZE + x));
    } else {
        ret.push((false, x)); // top
    }
    if y + 1 < SKY_NEIGHBOR_SIZE {
        ret.push((true, (y + 1) * SKY_NEIGHBOR_SIZE + x));
    } else {
        ret.push((false, SKY_NEIGHBOR_SIZE + x)); // bottom
    }
    if x > 0 {
        ret.push((true, y * SKY_NEIGHBOR_SIZE + x - 1));
    } else {
        ret.push((false, 2 * SKY_NEIGHBOR_SIZE + y)); // left
    }
    if x + 1 < SKY_NEIGHBOR_SIZE {
        ret.push((true, y * SKY_NEIGHBOR_SIZE + x + 1));
    } else {
        ret.push((false, 3 * SKY_NEIGHBOR_SIZE + y)); // right
    }
    ret
}

fn outer_neighbours(i: usize) -> Vec<(bool, usize)> {
    let side = i / SKY_NEIGHBOR_SIZE;
    let pos = i % SKY_NEIGHBOR_SIZE;
    let mut ret = Vec::with_capacity(3);
    if pos > 0 {
        ret.push((false, i - 1));
    }
    if pos + 1 < SKY_NEIGHBOR_SIZE {
        ret.push((false, i + 1));
    }
    let inner = match side {
        0 => pos,                         // top
        1 => 8 * SKY_NEIGHBOR_SIZE + pos, // bottom
        2 => pos * SKY_NEIGHBOR_SIZE,     // left
        3 => pos * SKY_NEIGHBOR_SIZE + 8, // right
        _ => unreachable!(),
    };
    ret.push((true, inner));
    ret
}

/// Solve Sky Neighbors and return all values which are forced by the clues.
pub fn solve_sky_neighbor(problem: &Problem) -> Option<(Vec<Vec<Option<i32>>>, Vec<Option<i32>>)> {
    if !valid_problem(problem) {
        return None;
    }

    let mut solver = Solver::new();
    let inner = &solver.int_var_2d((SKY_NEIGHBOR_SIZE, SKY_NEIGHBOR_SIZE), 1, 3);
    let outer = &solver.int_var_1d(OUTER_SIZE, 1, 3);
    solver.add_answer_key_int(inner);
    solver.add_answer_key_int(outer);

    for y in 0..SKY_NEIGHBOR_SIZE {
        for x in 0..SKY_NEIGHBOR_SIZE {
            if let Some(n) = problem.inner_givens[y][x] {
                solver.add_expr(inner.at((y, x)).eq(n));
            }
        }
    }
    for (i, n) in problem.outer_givens.iter().enumerate() {
        if let Some(n) = n {
            solver.add_expr(outer.at(i).eq(*n));
        }
    }

    // Each inner row and column contains each digit exactly three times.
    for n in 1..=3 {
        for y in 0..SKY_NEIGHBOR_SIZE {
            solver.add_expr(inner.slice_fixed_y((y, ..)).eq(n).count_true().eq(3));
        }
        for x in 0..SKY_NEIGHBOR_SIZE {
            solver.add_expr(inner.slice_fixed_x((.., x)).eq(n).count_true().eq(3));
        }
    }

    // Neighbors rule on the complete 117-cell graph.
    for y in 0..SKY_NEIGHBOR_SIZE {
        for x in 0..SKY_NEIGHBOR_SIZE {
            let value = inner.at((y, x));
            let same = neighbours_of(y, x)
                .into_iter()
                .map(|(is_inner, i)| {
                    if is_inner {
                        inner
                            .at((i / SKY_NEIGHBOR_SIZE, i % SKY_NEIGHBOR_SIZE))
                            .eq(value.clone())
                    } else {
                        outer.at(i).eq(value.clone())
                    }
                })
                .collect::<Vec<_>>();
            let same = count_true(&same);
            if problem.inner_outlined[y][x] {
                solver.add_expr(same.eq(0));
            } else {
                solver.add_expr(same.ge(1));
            }
        }
    }
    for i in 0..OUTER_SIZE {
        let value = outer.at(i);
        let same = outer_neighbours(i)
            .into_iter()
            .map(|(is_inner, j)| {
                if is_inner {
                    inner
                        .at((j / SKY_NEIGHBOR_SIZE, j % SKY_NEIGHBOR_SIZE))
                        .eq(value.clone())
                } else {
                    outer.at(j).eq(value.clone())
                }
            })
            .collect::<Vec<_>>();
        let same = count_true(&same);
        if problem.outer_outlined[i] {
            solver.add_expr(same.eq(0));
        } else {
            solver.add_expr(same.ge(1));
        }
    }

    // Visibility clues.  A building is visible iff it is strictly taller
    // than every building before it in the viewing direction.
    for side in 0..4 {
        for pos in 0..SKY_NEIGHBOR_SIZE {
            let visible = &solver.bool_var_1d(SKY_NEIGHBOR_SIZE);
            for p in 0..SKY_NEIGHBOR_SIZE {
                let (y, x) = match side {
                    0 => (p, pos),     // top
                    1 => (8 - p, pos), // bottom
                    2 => (pos, p),     // left
                    3 => (pos, 8 - p), // right
                    _ => unreachable!(),
                };
                if p == 0 {
                    solver.add_expr(visible.at(p));
                } else {
                    let mut conditions = Vec::with_capacity(p);
                    for q in 0..p {
                        let (py, px) = match side {
                            0 => (q, pos),
                            1 => (8 - q, pos),
                            2 => (pos, q),
                            3 => (pos, 8 - q),
                            _ => unreachable!(),
                        };
                        conditions.push(inner.at((py, px)).lt(inner.at((y, x))));
                    }
                    solver.add_expr(visible.at(p).iff(all(&conditions)));
                }
            }
            let outer_index = side * SKY_NEIGHBOR_SIZE + pos;
            solver.add_expr(outer.at(outer_index).eq(visible.count_true()));
        }
    }

    let facts = solver.irrefutable_facts()?;
    Some((facts.get(inner), facts.get(outer)))
}

pub fn solve(problem: &Problem) -> Option<(Vec<Vec<Option<i32>>>, Vec<Option<i32>>)> {
    solve_sky_neighbor(problem)
}

struct SkyNeighborCombinator;

fn encode_optional(n: Option<i32>, out: &mut Vec<u8>) -> Option<()> {
    match n {
        None => out.push(b'.'),
        Some(n @ 1..=3) => out.push(b'0' + n as u8),
        _ => return None,
    }
    Some(())
}

fn decode_optional(c: u8) -> Option<Option<i32>> {
    match c {
        b'.' => Some(None),
        b'1'..=b'3' => Some(Some((c - b'0') as i32)),
        _ => None,
    }
}

impl Combinator<Problem> for SkyNeighborCombinator {
    fn serialize(&self, _: &Context, input: &[Problem]) -> Option<(usize, Vec<u8>)> {
        let problem = input.first()?;
        if !valid_problem(problem) {
            return None;
        }
        let mut out = Vec::with_capacity(204);
        for n in problem.inner_givens.iter().flatten() {
            encode_optional(*n, &mut out)?;
        }
        out.push(b'/');
        for n in &problem.outer_givens {
            encode_optional(*n, &mut out)?;
        }
        out.push(b'/');
        for &gray in problem.inner_outlined.iter().flatten() {
            out.push(if gray { b'1' } else { b'0' });
        }
        out.push(b'/');
        for &gray in &problem.outer_outlined {
            out.push(if gray { b'1' } else { b'0' });
        }
        Some((1, out))
    }

    fn deserialize(&self, _: &Context, input: &[u8]) -> Option<(usize, Vec<Problem>)> {
        let expected = SKY_NEIGHBOR_SIZE * SKY_NEIGHBOR_SIZE
            + 1
            + OUTER_SIZE
            + 1
            + SKY_NEIGHBOR_SIZE * SKY_NEIGHBOR_SIZE
            + 1
            + OUTER_SIZE;
        if input.len() < expected {
            return None;
        }
        let mut at = 0;
        let mut next_grid = || {
            let mut result = vec![vec![]; SKY_NEIGHBOR_SIZE];
            for row in &mut result {
                for _ in 0..SKY_NEIGHBOR_SIZE {
                    row.push(decode_optional(input[at])?);
                    at += 1;
                }
            }
            Some(result)
        };
        let inner_givens = next_grid()?;
        if input[at] != b'/' {
            return None;
        }
        at += 1;
        let mut outer_givens = Vec::with_capacity(OUTER_SIZE);
        for _ in 0..OUTER_SIZE {
            outer_givens.push(decode_optional(input[at])?);
            at += 1;
        }
        if input[at] != b'/' {
            return None;
        }
        at += 1;
        let mut inner_outlined = vec![vec![false; SKY_NEIGHBOR_SIZE]; SKY_NEIGHBOR_SIZE];
        for row in &mut inner_outlined {
            for gray in row {
                *gray = match input[at] {
                    b'0' => false,
                    b'1' => true,
                    _ => return None,
                };
                at += 1;
            }
        }
        if input[at] != b'/' {
            return None;
        }
        at += 1;
        let mut outer_outlined = Vec::with_capacity(OUTER_SIZE);
        for _ in 0..OUTER_SIZE {
            outer_outlined.push(match input[at] {
                b'0' => false,
                b'1' => true,
                _ => return None,
            });
            at += 1;
        }
        let problem = Problem {
            inner_givens,
            outer_givens,
            inner_outlined,
            outer_outlined,
        };
        valid_problem(&problem).then_some((at, vec![problem]))
    }
}

fn combinator() -> impl Combinator<Problem> {
    Size::new(SkyNeighborCombinator)
}

fn compact_bytes(input: &str) -> Vec<u8> {
    input
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b';' && *c != b',' && *c != b'|')
        .collect()
}

fn compact_len(input: &str) -> usize {
    compact_bytes(input).len()
}

// PuzzLink links copied from a browser may percent-encode separators (most
// commonly the semicolons between gray rows). Decode those escapes before
// splitting the payload, while leaving malformed escapes untouched so normal
// validation still rejects them.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push(((hi << 4) | lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_grid(input: &str) -> Option<Vec<Vec<Option<i32>>>> {
    let bytes = compact_bytes(input);
    if bytes.len() != SKY_NEIGHBOR_SIZE * SKY_NEIGHBOR_SIZE {
        return None;
    }
    let mut result = vec![vec![None; SKY_NEIGHBOR_SIZE]; SKY_NEIGHBOR_SIZE];
    for (i, c) in bytes.into_iter().enumerate() {
        result[i / SKY_NEIGHBOR_SIZE][i % SKY_NEIGHBOR_SIZE] = match c {
            b'.' | b'0' | b'-' | b'_' => None,
            b'1'..=b'3' => Some((c - b'0') as i32),
            _ => return None,
        };
    }
    Some(result)
}

fn parse_side(input: &str, len: usize) -> Option<Vec<Option<i32>>> {
    let bytes = compact_bytes(input);
    if bytes.len() != len {
        return None;
    }
    bytes
        .into_iter()
        .map(|c| match c {
            b'.' | b'0' | b'-' | b'_' => Some(None),
            b'1'..=b'3' => Some(Some((c - b'0') as i32)),
            _ => None,
        })
        .collect()
}

fn parse_gray_grid(input: &str) -> Option<Vec<Vec<bool>>> {
    let bytes = compact_bytes(input);
    if bytes.len() != SKY_NEIGHBOR_SIZE * SKY_NEIGHBOR_SIZE {
        return None;
    }
    let mut result = vec![vec![false; SKY_NEIGHBOR_SIZE]; SKY_NEIGHBOR_SIZE];
    for (i, c) in bytes.into_iter().enumerate() {
        result[i / SKY_NEIGHBOR_SIZE][i % SKY_NEIGHBOR_SIZE] = match c {
            b'0' | b'.' | b'-' | b'_' => false,
            b'1' | b'#' | b'g' | b'G' | b'x' | b'X' => true,
            _ => return None,
        };
    }
    Some(result)
}

fn parse_gray_side(input: &str, len: usize) -> Option<Vec<bool>> {
    let bytes = compact_bytes(input);
    if bytes.len() != len {
        return None;
    }
    bytes
        .into_iter()
        .map(|c| match c {
            b'0' | b'.' | b'-' | b'_' => Some(false),
            b'1' | b'#' | b'g' | b'G' | b'x' | b'X' => Some(true),
            _ => None,
        })
        .collect()
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    if !valid_problem(problem) {
        return None;
    }
    problem_to_url_with_context(
        combinator(),
        "skyneighbors",
        problem.clone(),
        &Context::sized(SKY_NEIGHBOR_SIZE, SKY_NEIGHBOR_SIZE),
    )
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let body = percent_decode(strip_prefix(url)?);
    let mut parts = body.split('/');
    let kind = parts.next()?;
    if ![
        "skyneighbor",
        "skyneighbors",
        "sky-neighbor",
        "sky-neighbors",
        "skyneighbour",
        "skyneighbours",
        "sky-neighbour",
        "sky-neighbours",
    ]
    .contains(&kind)
    {
        return None;
    }
    // Sky-neighbors is deliberately fixed at 9x9.  Do not let the generic
    // Size combinator silently accept another dimension and then index a
    // differently shaped model.
    if parts.next()?.parse::<usize>().ok()? != SKY_NEIGHBOR_SIZE
        || parts.next()?.parse::<usize>().ok()? != SKY_NEIGHBOR_SIZE
    {
        return None;
    }
    let payload = parts.collect::<Vec<_>>();

    // The compact four-layer form is the native cspuz/pzpr representation:
    // inner givens / outside values / inner gray / outside gray.
    if payload.len() >= 4
        && compact_len(payload[0]) == SKY_NEIGHBOR_SIZE * SKY_NEIGHBOR_SIZE
        && compact_len(payload[1]) == OUTER_SIZE
        && compact_len(payload[2]) == SKY_NEIGHBOR_SIZE * SKY_NEIGHBOR_SIZE
        && compact_len(payload[3]) == OUTER_SIZE
    {
        let problem = Problem {
            inner_givens: parse_grid(payload[0])?,
            outer_givens: parse_side(payload[1], OUTER_SIZE)?,
            inner_outlined: parse_gray_grid(payload[2])?,
            outer_outlined: parse_gray_side(payload[3], OUTER_SIZE)?,
        };
        return valid_problem(&problem).then_some(problem);
    }

    // Paper-puzzle imports commonly use six or ten payload layers:
    // inner / inner-gray / top / bottom / left / right [/ four gray masks].
    // Accept this form as well so links copied from the reference examples
    // can be solved directly by the backend.
    if payload.len() != 6 && payload.len() != 10 {
        return None;
    }
    let inner_givens = parse_grid(payload[0])?;
    let inner_outlined = parse_gray_grid(payload[1])?;
    let mut outer_givens = Vec::with_capacity(OUTER_SIZE);
    for side in &payload[2..6] {
        outer_givens.extend(parse_side(side, SKY_NEIGHBOR_SIZE)?);
    }
    let outer_outlined = if payload.len() == 10 {
        let mut result = Vec::with_capacity(OUTER_SIZE);
        for side in &payload[6..10] {
            result.extend(parse_gray_side(side, SKY_NEIGHBOR_SIZE)?);
        }
        result
    } else {
        vec![false; OUTER_SIZE]
    };
    let problem = Problem {
        inner_givens,
        outer_givens,
        inner_outlined,
        outer_outlined,
    };
    valid_problem(&problem).then_some(problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUZZLE_22: &str = "https://puzz.link/p?sky-neighbor/9/9/..........1.............................2.............................3........../.G..GG.GG;GGGG.G...;.G.G...GG;.G..G..G.;.GG....G.;.G.G....G;.G.GGGG.G;GGG....GG;.G.G.GGGG/212221313/212223121/213211232/221223121/010001111/010001111/011100010/001001111";

    fn empty_problem() -> Problem {
        Problem {
            inner_givens: vec![vec![None; 9]; 9],
            outer_givens: vec![None; 36],
            inner_outlined: vec![vec![false; 9]; 9],
            outer_outlined: vec![false; 36],
        }
    }

    #[test]
    fn serializer_roundtrip() {
        let problem = empty_problem();
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(
            url,
            format!(
                "https://puzz.link/p?skyneighbor/9/9/{}/{}/{}/{}",
                ".".repeat(81),
                ".".repeat(36),
                "0".repeat(81),
                "0".repeat(36)
            )
        );
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn rejects_bad_shape_and_values() {
        let mut problem = empty_problem();
        problem.outer_givens.pop();
        assert!(serialize_problem(&problem).is_none());
        let mut problem = empty_problem();
        problem.inner_givens[0][0] = Some(4);
        assert!(serialize_problem(&problem).is_none());

        assert!(deserialize_problem(
            "https://puzz.link/p?skyneighbor/8/9/................................................................................./..................................../000000000000000000000000000000000000000000000000000000000000000000000000000000000/000000000000000000000000000000000000"
        )
        .is_none());
    }

    #[test]
    fn imports_and_solves_published_puzzle_22() {
        let problem = deserialize_problem(PUZZLE_22).unwrap();
        assert_eq!(problem.inner_givens[1][1], Some(1));
        assert_eq!(problem.inner_givens[4][4], Some(2));
        assert_eq!(problem.inner_givens[7][7], Some(3));
        assert_eq!(
            problem.outer_givens[..9],
            [
                Some(2),
                Some(1),
                Some(2),
                Some(2),
                Some(2),
                Some(1),
                Some(3),
                Some(1),
                Some(3),
            ]
        );

        let expected = [
            "232213131",
            "313132122",
            "121233213",
            "131312232",
            "312321312",
            "323121321",
            "213213123",
            "121332231",
            "232121313",
        ];
        let (inner, outer) = solve_sky_neighbor(&problem).unwrap();
        for y in 0..SKY_NEIGHBOR_SIZE {
            for x in 0..SKY_NEIGHBOR_SIZE {
                assert_eq!(inner[y][x], Some((expected[y].as_bytes()[x] - b'0') as i32));
            }
        }
        assert_eq!(outer, problem.outer_givens);

        for alias in [
            "skyneighbor",
            "skyneighbors",
            "sky-neighbor",
            "sky-neighbors",
            "skyneighbour",
            "skyneighbours",
            "sky-neighbour",
            "sky-neighbours",
        ] {
            let alias_url = PUZZLE_22.replacen("sky-neighbor", alias, 1);
            assert_eq!(deserialize_problem(&alias_url), Some(problem.clone()));
        }
    }

    #[test]
    fn accepts_percent_encoded_separators() {
        let encoded = PUZZLE_22.replace(';', "%3B");
        let problem = deserialize_problem(&encoded).unwrap();
        assert_eq!(problem.inner_givens[1][1], Some(1));
        assert_eq!(problem.inner_outlined[0][1], false);
        assert_eq!(problem.inner_outlined[0][2], true);
    }
}
