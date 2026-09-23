use crate::util;
use cspuz_rs::solver::{consecutive_prefix_true, sum, IntVarArray2D, Solver};

/// Direction values used in the answer grid: 0 is a numbered clue cell, and
/// 1..=4 are up, down, left and right respectively.
fn add_fourwinds_constraints(
    solver: &mut Solver,
    clues: &[Vec<Option<i32>>],
) -> Option<(usize, usize, IntVarArray2D)> {
    let (h, w) = util::infer_shape(clues);
    if h == 0 || w == 0 || clues.iter().any(|row| row.len() != w) {
        return None;
    }
    let dir = solver.int_var_2d((h, w), 0, 4);
    solver.add_answer_key_int(&dir);

    // Numbered cells are occupied by clues, not arrows.
    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_some() {
                solver.add_expr(dir.at((y, x)).eq(0));
            }
        }
    }

    // Every empty cell must be covered by exactly one arrow.
    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_none() {
                solver.add_expr(dir.at((y, x)).ne(0));
            }
        }
    }

    // An arrow cell must be connected backwards to a numbered cell.  This
    // prevents arrows from starting in the middle of an otherwise empty run.
    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_some() {
                continue;
            }
            let d = dir.at((y, x));
            // An up arrow's origin clue must be somewhere below.
            if y + 1 == h {
                solver.add_expr(d.ne(1));
            } else if clues[y + 1][x].is_none() {
                solver.add_expr(d.eq(1).imp(dir.at((y + 1, x)).eq(1)));
            }
            // A down arrow's origin clue must be somewhere above.
            if y == 0 {
                solver.add_expr(d.ne(2));
            } else if clues[y - 1][x].is_none() {
                solver.add_expr(d.eq(2).imp(dir.at((y - 1, x)).eq(2)));
            }
            // A left arrow's origin clue must be somewhere to the right.
            if x + 1 == w {
                solver.add_expr(d.ne(3));
            } else if clues[y][x + 1].is_none() {
                solver.add_expr(d.eq(3).imp(dir.at((y, x + 1)).eq(3)));
            }
            // A right arrow's origin clue must be somewhere to the left.
            if x == 0 {
                solver.add_expr(d.ne(4));
            } else if clues[y][x - 1].is_none() {
                solver.add_expr(d.eq(4).imp(dir.at((y, x - 1)).eq(4)));
            }
        }
    }

    // For each numbered cell, count the consecutive arrow cells beginning at
    // each adjacent edge.  A clue stops a ray, as required by the rules.
    for y in 0..h {
        for x in 0..w {
            let Some(n) = clues[y][x] else { continue };
            let mut lengths = vec![];
            lengths.push(consecutive_prefix_true(
                (0..y)
                    .rev()
                    .map(|yy| dir.at((yy, x)).eq(1))
                    .collect::<Vec<_>>(),
            ));
            lengths.push(consecutive_prefix_true(
                ((y + 1)..h)
                    .map(|yy| dir.at((yy, x)).eq(2))
                    .collect::<Vec<_>>(),
            ));
            lengths.push(consecutive_prefix_true(
                (0..x)
                    .rev()
                    .map(|xx| dir.at((y, xx)).eq(3))
                    .collect::<Vec<_>>(),
            ));
            lengths.push(consecutive_prefix_true(
                ((x + 1)..w)
                    .map(|xx| dir.at((y, xx)).eq(4))
                    .collect::<Vec<_>>(),
            ));
            solver.add_expr(sum(lengths).eq(n));
        }
    }

    Some((h, w, dir))
}

pub fn solve_fourwinds(clues: &[Vec<Option<i32>>]) -> Option<Vec<Vec<Option<i32>>>> {
    if clues.is_empty() || clues[0].is_empty() {
        return None;
    }
    // Four Winds clues are non-negative totals.  A negative value is not a
    // wildcard in this puzzle (an absent clue is represented by `None`).
    if clues
        .iter()
        .flatten()
        .any(|clue| clue.is_some_and(|n| n < 0))
    {
        return None;
    }

    let mut solver = Solver::new();
    let (_, _, dir) = add_fourwinds_constraints(&mut solver, clues)?;
    solver.irrefutable_facts().map(|f| f.get(&dir))
}

pub type Problem = Vec<Vec<Option<i32>>>;

fn hex_digit(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'f' => Some((c - b'a' + 10) as i32),
        b'A'..=b'F' => Some((c - b'A' + 10) as i32),
        _ => None,
    }
}

fn parse_hex(bytes: &[u8]) -> Option<i32> {
    if bytes.is_empty() {
        return None;
    }
    bytes
        .iter()
        .try_fold(0i32, |value, &digit| Some(value * 16 + hex_digit(digit)?))
}

/// Decode pzpr's arrow-number16 cell stream.  Lowercase letters represent a
/// run of empty cells; all other records carry one cell.  The arrow direction
/// embedded in each record is answer data and is ignored here, because only
/// the number clues belong to the problem.
fn decode_cells(payload: &[u8], width: usize, height: usize) -> Option<Problem> {
    let total = width.checked_mul(height)?;
    let mut cells = vec![None; total];
    let mut index = 0usize;
    let mut pos = 0usize;
    while index < total && pos < payload.len() {
        let code = payload[pos];
        if (b'a'..=b'z').contains(&code) {
            let skip = (code - b'a' + 1) as usize;
            index = index.checked_add(skip)?;
            pos += 1;
            continue;
        }

        let (count, consumed) = match code {
            b'+' => (None, 1),
            b'0'..=b'4' => {
                if pos + 1 >= payload.len() {
                    return None;
                }
                let count = if payload[pos + 1] == b'.' {
                    None
                } else {
                    Some(hex_digit(payload[pos + 1])?)
                };
                (count, 2)
            }
            b'5'..=b'9' => {
                if pos + 2 >= payload.len() {
                    return None;
                }
                let count = parse_hex(&payload[pos + 1..pos + 3])?;
                (Some(count), 3)
            }
            b'-' => {
                if pos + 4 >= payload.len() {
                    return None;
                }
                let count = parse_hex(&payload[pos + 2..pos + 5])?;
                // 0xfff is the "unknown number" placeholder used to keep an
                // unfinished editor board round-trippable.
                (if count == 0xfff { None } else { Some(count) }, 5)
            }
            _ => return None,
        };

        if index >= total {
            return None;
        }
        cells[index] = count;
        index += 1;
        pos += consumed;
    }

    Some(cells.chunks(width).map(|row| row.to_vec()).collect())
}

fn flush_skips(payload: &mut String, skipped: &mut usize) {
    while *skipped > 0 {
        let chunk = (*skipped).min(26);
        payload.push((b'a' + chunk as u8 - 1) as char);
        *skipped -= chunk;
    }
}

fn encode_cells(problem: &Problem) -> Option<String> {
    let (height, width) = util::infer_shape(problem);
    if height == 0 || width == 0 {
        return None;
    }
    let mut payload = String::new();
    let mut skipped = 0usize;
    for row in problem {
        for &clue in row {
            match clue {
                None => skipped += 1,
                Some(n) if (0..16).contains(&n) => {
                    flush_skips(&mut payload, &mut skipped);
                    payload.push_str(&format!("0{n:x}"));
                }
                Some(n) if (16..256).contains(&n) => {
                    flush_skips(&mut payload, &mut skipped);
                    payload.push_str(&format!("5{n:02x}"));
                }
                Some(n) if (0..4096).contains(&n) => {
                    flush_skips(&mut payload, &mut skipped);
                    payload.push_str(&format!("-0{n:03x}"));
                }
                _ => return None,
            }
        }
    }
    flush_skips(&mut payload, &mut skipped);
    Some(payload)
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    if problem.is_empty() || problem[0].is_empty() {
        return None;
    }
    let (height, width) = util::infer_shape(problem);
    if height == 0 || width == 0 || problem.iter().any(|row| row.len() != width) {
        return None;
    }
    let payload = encode_cells(problem)?;
    Some(format!(
        "https://puzz.link/p?fourwinds/{width}/{height}/{payload}"
    ))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let payload = url.split('?').nth(1)?;
    let mut parts = payload.split('/');
    if parts.next()? != "fourwinds" {
        return None;
    }
    let width = parts.next()?.parse().ok()?;
    let height = parts.next()?.parse().ok()?;
    decode_cells(parts.next().unwrap_or("").as_bytes(), width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clues(problem: &[Vec<i32>]) -> Problem {
        problem
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&n| if n < 0 { None } else { Some(n) })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn solves_fourwinds_example() {
        // Corner clues force rays along the borders; the center 0 forbids any
        // ray pointing into it.  The solution is unique.
        let problem = clues(&[vec![4, -1, -1], vec![-1, 0, -1], vec![-1, -1, 2]]);
        let ans = solve_fourwinds(&problem).unwrap();
        assert_eq!(
            ans,
            vec![
                vec![Some(0), Some(4), Some(4)],
                vec![Some(2), Some(0), Some(1)],
                vec![Some(2), Some(3), Some(0)]
            ]
        );
    }

    #[test]
    fn solves_wpc_2016_round1_puzzle1() {
        // WPF Puzzle GP 2016, Round 1, Competitive, puzzle 1 (19 points).
        let problem = clues(&[
            vec![-1, -1, 3, -1, 5, -1, -1, 3, -1, -1],
            vec![-1, -1, -1, -1, -1, 5, -1, -1, -1, -1],
            vec![3, -1, -1, -1, -1, -1, -1, 4, -1, 2],
            vec![-1, -1, -1, 5, -1, -1, -1, -1, -1, -1],
            vec![-1, -1, -1, -1, -1, -1, -1, -1, 5, -1],
            vec![-1, 8, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![-1, -1, -1, -1, -1, -1, 6, -1, -1, -1],
            vec![2, -1, 8, -1, -1, -1, -1, -1, -1, 8],
            vec![-1, -1, -1, -1, 9, -1, -1, -1, -1, -1],
            vec![-1, -1, 2, -1, -1, 2, -1, 2, -1, -1],
        ]);
        let ans = solve_fourwinds(&problem).expect("official puzzle must have a solution");
        assert_eq!(ans.len(), 10);
        assert_eq!(ans[0].len(), 10);
    }

    #[test]
    fn solves_wpc_2016_round1_puzzle2() {
        // WPF Puzzle GP 2016, Round 1, Competitive, puzzle 2 (14 points).
        // The 9x9 clue grid was transcribed from the competition booklet and
        // cross-checked against the official solution booklet.
        let problem = clues(&[
            vec![2, -1, 8, -1, -1, 2, -1, -1, 2],
            vec![-1, 2, -1, -1, 4, -1, -1, 2, -1],
            vec![-1; 9],
            vec![-1, -1, -1, -1, -1, 4, -1, -1, -1],
            vec![4, -1, -1, -1, 4, -1, -1, -1, 4],
            vec![-1, -1, -1, 4, -1, -1, -1, -1, -1],
            vec![-1; 9],
            vec![-1, 2, -1, -1, 2, -1, -1, 2, -1],
            vec![4, -1, -1, 2, -1, -1, 6, -1, 2],
        ]);
        let ans = solve_fourwinds(&problem).expect("official puzzle must have a solution");
        assert_eq!(ans.len(), 9);
        assert_eq!(ans[0].len(), 9);
    }

    #[test]
    fn wpc_2016_puzzle1_has_unique_solution() {
        // Enumerate all solutions of the WPC 2016 puzzle 1 clue grid to make
        // sure the rules above admit exactly the official solution.
        let problem = clues(&[
            vec![-1, -1, 3, -1, 5, -1, -1, 3, -1, -1],
            vec![-1, -1, -1, -1, -1, 5, -1, -1, -1, -1],
            vec![3, -1, -1, -1, -1, -1, -1, 4, -1, 2],
            vec![-1, -1, -1, 5, -1, -1, -1, -1, -1, -1],
            vec![-1, -1, -1, -1, -1, -1, -1, -1, 5, -1],
            vec![-1, 8, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![-1, -1, -1, -1, -1, -1, 6, -1, -1, -1],
            vec![2, -1, 8, -1, -1, -1, -1, -1, -1, 8],
            vec![-1, -1, -1, -1, 9, -1, -1, -1, -1, -1],
            vec![-1, -1, 2, -1, -1, 2, -1, 2, -1, -1],
        ]);
        let count = {
            let mut solver = Solver::new();
            add_fourwinds_constraints(&mut solver, &problem).unwrap();
            solver.answer_iter().count()
        };
        assert_eq!(count, 1);
    }

    #[test]
    fn wpc_2016_puzzle2_has_unique_solution() {
        // Same enumeration check for the official puzzle 2 clue grid.
        let problem = clues(&[
            vec![2, -1, 8, -1, -1, 2, -1, -1, 2],
            vec![-1, 2, -1, -1, 4, -1, -1, 2, -1],
            vec![-1; 9],
            vec![-1, -1, -1, -1, -1, 4, -1, -1, -1],
            vec![4, -1, -1, -1, 4, -1, -1, -1, 4],
            vec![-1, -1, -1, 4, -1, -1, -1, -1, -1],
            vec![-1; 9],
            vec![-1, 2, -1, -1, 2, -1, -1, 2, -1],
            vec![4, -1, -1, 2, -1, -1, 6, -1, 2],
        ]);
        let count = {
            let mut solver = Solver::new();
            add_fourwinds_constraints(&mut solver, &problem).unwrap();
            solver.answer_iter().count()
        };
        assert_eq!(count, 1);
    }

    #[test]
    fn wpc_2016_puzzle1_solution_satisfies_all_clues() {
        let problem = clues(&[
            vec![-1, -1, 3, -1, 5, -1, -1, 3, -1, -1],
            vec![-1, -1, -1, -1, -1, 5, -1, -1, -1, -1],
            vec![3, -1, -1, -1, -1, -1, -1, 4, -1, 2],
            vec![-1, -1, -1, 5, -1, -1, -1, -1, -1, -1],
            vec![-1, -1, -1, -1, -1, -1, -1, -1, 5, -1],
            vec![-1, 8, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![-1, -1, -1, -1, -1, -1, 6, -1, -1, -1],
            vec![2, -1, 8, -1, -1, -1, -1, -1, -1, 8],
            vec![-1, -1, -1, -1, 9, -1, -1, -1, -1, -1],
            vec![-1, -1, 2, -1, -1, 2, -1, 2, -1, -1],
        ]);
        let ans = solve_fourwinds(&problem).unwrap();
        let (h, w) = (ans.len(), ans[0].len());
        let steps = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for y in 0..h {
            for x in 0..w {
                let Some(n) = problem[y][x] else { continue };
                assert_eq!(ans[y][x], Some(0), "clue cell must not hold an arrow");
                let total: i32 = (1..=4)
                    .map(|d| {
                        let (mut yy, mut xx) =
                            (y as isize + steps[d - 1].0, x as isize + steps[d - 1].1);
                        let mut len = 0;
                        while yy >= 0
                            && yy < h as isize
                            && xx >= 0
                            && xx < w as isize
                            && ans[yy as usize][xx as usize] == Some(d as i32)
                        {
                            len += 1;
                            yy += steps[d - 1].0;
                            xx += steps[d - 1].1;
                        }
                        len
                    })
                    .sum();
                assert_eq!(total, n, "clue total mismatch at ({y},{x})");
            }
        }
        for y in 0..h {
            for x in 0..w {
                if problem[y][x].is_none() {
                    assert!(
                        matches!(ans[y][x], Some(1..=4)),
                        "empty cell ({y},{x}) must be covered by an arrow"
                    );
                }
            }
        }
    }

    #[test]
    fn rejects_clueless_boards() {
        // Every ray must start at a numbered cell, so a board without clues
        // has no valid assignment.
        let problem = clues(&[vec![-1; 3], vec![-1; 3], vec![-1; 3]]);
        assert!(solve_fourwinds(&problem).is_none());
    }

    #[test]
    fn test_fourwinds_serializer_roundtrip() {
        let problem = clues(&[vec![1, -1, 12], vec![-1, 200, -1]]);
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn decodes_pzpr_arrow_number16_url() {
        let problem = deserialize_problem("https://puzz.link/p?fourwinds/3/3/04c00c02").unwrap();
        assert_eq!(problem[0][0], Some(4));
        assert!(problem[0][1].is_none());
        assert_eq!(problem[1][1], Some(0));
        assert_eq!(problem[2][2], Some(2));
    }

    #[test]
    fn decodes_pzpr_url_with_answer_arrows() {
        // Direction digits embedded next to numbers are answer data and must
        // be ignored; only the numbers are read.
        let problem = deserialize_problem("https://puzz.link/p?fourwinds/3/3/14a42a00c").unwrap();
        assert_eq!(problem[0][0], Some(4));
        assert!(problem[0][1].is_none());
        assert_eq!(problem[0][2], Some(2));
        assert_eq!(problem[1][1], Some(0));
        assert!(problem[2][2].is_none());
    }

    #[test]
    fn rejects_negative_clues() {
        let problem: Problem = vec![vec![Some(-2), None], vec![None, Some(0)]];
        assert!(solve_fourwinds(&problem).is_none());
        assert!(serialize_problem(&problem).is_none());
    }

    #[test]
    fn rejects_empty_or_ragged_grids() {
        let empty: Problem = vec![];
        assert!(solve_fourwinds(&empty).is_none());
        assert!(serialize_problem(&empty).is_none());
        let ragged = clues(&[vec![-1, -1], vec![-1]]);
        assert!(solve_fourwinds(&ragged).is_none());
        assert!(serialize_problem(&ragged).is_none());
    }
}
