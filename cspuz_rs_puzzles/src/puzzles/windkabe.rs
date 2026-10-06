use crate::util;
use cspuz_rs::graph;
use cspuz_rs::solver::{
    consecutive_prefix_true, sum, BoolExprArray2D, IntVarArray2D, FALSE, Solver,
};

/// Wind Kabe: Four Winds arrows plus a "kabe" (wall) condition.
///
/// From every numbered cell, rays extend in the four cardinal directions;
/// the number equals the total count of cells covered by its rays, and rays
/// never enter other numbered cells.  Every cell not covered by a ray is
/// black (part of the wall).  The black cells must form a single connected
/// region, and no 2x2 block may be entirely black.
///
/// Direction values used in the answer grid: 0 is a black (uncovered) cell,
/// and 1..=4 are up, down, left and right respectively.  Numbered cells are
/// also 0 but are excluded from the wall.
fn add_windkabe_constraints(
    solver: &mut Solver,
    clues: &[Vec<Option<i32>>],
    dir: &IntVarArray2D,
) {
    let (h, w) = util::infer_shape(clues);

    // Numbered cells are occupied by clues, not arrows.
    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_some() {
                solver.add_expr(dir.at((y, x)).eq(0));
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

    // The wall consists of every empty cell not covered by an arrow.
    // Numbered cells are never part of the wall.
    let is_black: BoolExprArray2D = {
        let mut data = vec![];
        for y in 0..h {
            for x in 0..w {
                if clues[y][x].is_none() {
                    data.push(dir.at((y, x)).eq(0));
                } else {
                    data.push(FALSE);
                }
            }
        }
        BoolExprArray2D::new((h, w), data)
    };

    // The wall must be connected.
    graph::active_vertices_connected_2d(solver, &is_black);

    // No 2x2 block may be entirely black.
    solver.add_expr(!is_black.conv2d_and((2, 2)));
}

pub fn solve_windkabe(clues: &[Vec<Option<i32>>]) -> Option<Vec<Vec<Option<i32>>>> {
    if clues.is_empty() || clues[0].is_empty() {
        return None;
    }
    let (h, w) = util::infer_shape(clues);
    if h == 0 || w == 0 || clues.iter().any(|row| row.len() != w) {
        return None;
    }
    // Wind Kabe clues are non-negative totals.  A negative value is not a
    // wildcard in this puzzle (an absent clue is represented by `None`).
    if clues
        .iter()
        .flatten()
        .any(|clue| clue.is_some_and(|n| n < 0))
    {
        return None;
    }

    let mut solver = Solver::new();
    let dir = &solver.int_var_2d((h, w), 0, 4);
    solver.add_answer_key_int(dir);
    add_windkabe_constraints(&mut solver, clues, dir);

    solver.irrefutable_facts().map(|f| f.get(dir))
}

pub fn enumerate_answers_windkabe(
    clues: &[Vec<Option<i32>>],
    num_max_answers: usize,
) -> Vec<Vec<Vec<i32>>> {
    let (h, w) = util::infer_shape(clues);

    let mut solver = Solver::new();
    let dir = &solver.int_var_2d((h, w), 0, 4);
    solver.add_answer_key_int(dir);
    add_windkabe_constraints(&mut solver, clues, dir);

    solver
        .answer_iter()
        .take(num_max_answers)
        .map(|f| f.get_unwrap(dir))
        .collect()
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
        "https://puzz.link/p?windkabe/{width}/{height}/{payload}"
    ))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let payload = url.split('?').nth(1)?;
    let mut parts = payload.split('/');
    if parts.next()? != "windkabe" {
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
    fn solves_windkabe_example() {
        // A single 1-clue in the top-middle of a 3x3 grid has a unique
        // solution: the ray must go down (the side rays would leave the wall
        // disconnected), and the remaining 7 cells form a connected ring.
        let problem = clues(&[vec![-1, 1, -1], vec![-1, -1, -1], vec![-1, -1, -1]]);
        let answers = enumerate_answers_windkabe(&problem, 8);
        assert_eq!(answers.len(), 1);
        assert_eq!(
            answers[0],
            vec![
                vec![0, 0, 0],
                vec![0, 2, 0],
                vec![0, 0, 0]
            ]
        );
    }

    #[test]
    fn solves_windkabe_with_0_clue() {
        // A single 0-clue in the middle of a 3x3 grid emits no ray; the
        // remaining 8 empty cells form a connected ring (no 2x2 violation
        // is possible since every 2x2 contains the white clue).
        let problem = clues(&[vec![-1, -1, -1], vec![-1, 0, -1], vec![-1, -1, -1]]);
        let answers = enumerate_answers_windkabe(&problem, 8);
        assert_eq!(answers.len(), 1);
        assert_eq!(
            answers[0],
            vec![
                vec![0, 0, 0],
                vec![0, 0, 0],
                vec![0, 0, 0]
            ]
        );
    }

    #[test]
    fn test_windkabe_serializer() {
        let problem = vec![vec![Some(1), None], vec![None, Some(0)]];
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn rejects_negative_clues() {
        let problem = vec![vec![Some(-1), None], vec![None, Some(0)]];
        assert!(solve_windkabe(&problem).is_none());
        assert!(serialize_problem(&problem).is_none());
    }

    #[test]
    fn rejects_empty_or_ragged_grids() {
        let empty: Problem = vec![];
        assert!(solve_windkabe(&empty).is_none());
        assert!(serialize_problem(&empty).is_none());
        let ragged = vec![vec![None, None], vec![None]];
        assert!(solve_windkabe(&ragged).is_none());
        assert!(serialize_problem(&ragged).is_none());
    }
}
