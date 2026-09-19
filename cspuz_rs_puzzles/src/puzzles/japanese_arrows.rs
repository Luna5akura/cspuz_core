use crate::util;
use cspuz_rs::solver::{count_true, Solver};

/// A Japanese Arrows cell consists of an arrow direction and an optional
/// given number. `direction` remains optional here only so an unfinished
/// pzpr editor URL can be decoded and round-tripped; every cell must have a
/// direction before `solve_japanese_arrows` will accept the problem.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArrowClue {
    pub direction: Option<i32>,
    pub count: Option<i32>,
}

pub type Problem = Vec<Vec<ArrowClue>>;

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
/// run of empty cells; all other records contain one cell and its arrow/count
/// pair.
fn decode_cells(payload: &[u8], width: usize, height: usize) -> Option<Problem> {
    let total = width.checked_mul(height)?;
    let mut cells = vec![ArrowClue::default(); total];
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

        let (direction, count, consumed) = match code {
            b'+' => (None, None, 1),
            b'0'..=b'4' => {
                if pos + 1 >= payload.len() {
                    return None;
                }
                let count = if payload[pos + 1] == b'.' {
                    None
                } else {
                    Some(hex_digit(payload[pos + 1])?)
                };
                let direction = match (code - b'0') as i32 {
                    0 => None,
                    value => Some(value),
                };
                (direction, count, 2)
            }
            b'5'..=b'9' => {
                if pos + 2 >= payload.len() {
                    return None;
                }
                let direction = match (code - b'5') as i32 {
                    0 => None,
                    value => Some(value),
                };
                let count = parse_hex(&payload[pos + 1..pos + 3])?;
                (direction, Some(count), 3)
            }
            b'-' => {
                if pos + 4 >= payload.len() {
                    return None;
                }
                let direction = hex_digit(payload[pos + 1])?;
                let count = parse_hex(&payload[pos + 2..pos + 5])?;
                (
                    Some(direction),
                    if count == 0xfff { None } else { Some(count) },
                    5,
                )
            }
            _ => return None,
        };

        if index >= total {
            return None;
        }
        cells[index] = ArrowClue { direction, count };
        index += 1;
        pos += consumed;
    }

    Some(cells.chunks(width).map(|row| row.to_vec()).collect())
}

fn encode_cell(clue: ArrowClue) -> Option<String> {
    // Keep the encoder permissive so unfinished editor boards can still be
    // round-tripped. `solve_japanese_arrows` performs the strict validation.
    let direction = clue.direction.unwrap_or(0);
    if !(0..=8).contains(&direction) {
        return None;
    }
    match clue.count {
        None if direction == 0 => None,
        None => Some(format!("-{direction:x}fff")),
        Some(count) if direction <= 4 && (0..16).contains(&count) => {
            Some(format!("{direction:x}{count:x}"))
        }
        Some(count) if direction <= 4 && (16..256).contains(&count) => {
            Some(format!("{:x}{count:02x}", direction + 5))
        }
        Some(count) if (0..4096).contains(&count) => Some(format!("-{direction:x}{count:03x}")),
        _ => None,
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
            let encoded = match (clue.direction, clue.count) {
                (None, None) => None,
                _ => Some(encode_cell(clue)?),
            };
            if let Some(encoded) = encoded {
                while skipped > 0 {
                    let chunk = skipped.min(26);
                    payload.push((b'a' + chunk as u8 - 1) as char);
                    skipped -= chunk;
                }
                payload.push_str(&encoded);
            } else {
                skipped += 1;
            }
        }
    }
    while skipped > 0 {
        let chunk = skipped.min(26);
        payload.push((b'a' + chunk as u8 - 1) as char);
        skipped -= chunk;
    }
    Some(payload)
}

fn step(direction: i32) -> Option<(isize, isize)> {
    Some(match direction {
        1 => (-1, 0),
        2 => (1, 0),
        3 => (0, -1),
        4 => (0, 1),
        5 => (-1, -1),
        6 => (-1, 1),
        7 => (1, -1),
        8 => (1, 1),
        _ => return None,
    })
}

pub fn solve_japanese_arrows(clues: &Problem) -> Option<Vec<Vec<Option<i32>>>> {
    let (height, width) = util::infer_shape(clues);
    if height == 0 || width == 0 || clues.iter().any(|row| row.len() != width) {
        return None;
    }
    // The competition rules give an arrow in every cell.  Missing arrows are
    // not unconstrained cells; they mean that the encoded problem is
    // incomplete and must not be solved as a different, weaker puzzle.
    if clues
        .iter()
        .flatten()
        .any(|clue| clue.direction.and_then(step).is_none())
    {
        return None;
    }
    // A value is a distinct-count on a ray that excludes its own cell.  The
    // rules do not prescribe a fixed digit set: a 4x4 board can reach 3, a
    // 5x5 board can reach 4, and so on.
    let max_number = height.max(width).saturating_sub(1).max(1) as i32;
    let mut solver = Solver::new();
    let numbers = &solver.int_var_2d((height, width), 1, max_number);
    solver.add_answer_key_int(numbers);

    for y in 0..height {
        for x in 0..width {
            if let Some(count) = clues[y][x].count {
                solver.add_expr(numbers.at((y, x)).eq(count));
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            let clue = clues[y][x];
            let Some(direction) = clue.direction else {
                continue;
            };
            let Some((dy, dx)) = step(direction) else {
                return None;
            };

            let mut ray = Vec::new();
            let (mut yy, mut xx) = (y as isize + dy, x as isize + dx);
            while yy >= 0 && yy < height as isize && xx >= 0 && xx < width as isize {
                ray.push((yy as usize, xx as usize));
                yy += dy;
                xx += dx;
            }

            let distinct = (1..=max_number)
                .map(|value| {
                    count_true(
                        ray.iter()
                            .map(|&(ry, rx)| numbers.at((ry, rx)).eq(value))
                            .collect::<Vec<_>>(),
                    )
                    .ge(1)
                })
                .collect::<Vec<_>>();
            let distinct_count = count_true(distinct);
            if let Some(count) = clue.count {
                solver.add_expr(distinct_count.eq(count));
            } else {
                solver.add_expr(distinct_count.eq(numbers.at((y, x))));
            }
        }
    }

    solver.irrefutable_facts().map(|facts| facts.get(numbers))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (height, width) = util::infer_shape(problem);
    if height == 0 || width == 0 || problem.iter().any(|row| row.len() != width) {
        return None;
    }
    let payload = encode_cells(problem)?;
    Some(format!(
        "https://puzz.link/p?japanese_arrows/{width}/{height}/{payload}"
    ))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let payload = url.split('?').nth(1)?;
    let mut parts = payload.split('/');
    if parts.next()? != "japanese_arrows" {
        return None;
    }
    let width = parts.next()?.parse().ok()?;
    let height = parts.next()?.parse().ok()?;
    decode_cells(parts.next().unwrap_or("").as_bytes(), width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_directional_distinct_count() {
        let directions = [[2, 2, 2], [4, 2, 3], [1, 1, 1]];
        let problem = (0..3)
            .map(|y| {
                (0..3)
                    .map(|x| ArrowClue {
                        direction: Some(directions[y][x]),
                        count: if (y, x) == (0, 0) { Some(2) } else { None },
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let answer = solve_japanese_arrows(&problem).unwrap();
        assert_eq!(answer.len(), 3);
        assert_eq!(answer[0].len(), 3);
    }

    #[test]
    fn round_trips_arrow_number_url() {
        let problem = vec![
            vec![
                ArrowClue {
                    direction: Some(1),
                    count: Some(2),
                },
                ArrowClue {
                    direction: Some(2),
                    count: None,
                },
            ],
            vec![
                ArrowClue {
                    direction: Some(4),
                    count: None,
                },
                ArrowClue {
                    direction: Some(3),
                    count: None,
                },
            ],
        ];
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn decodes_pzpr_skip_cells() {
        let problem = deserialize_problem("https://puzz.link/p?japanese_arrows/3/3/42h").unwrap();
        assert_eq!(problem[0][0].direction, Some(4));
        assert_eq!(problem[0][0].count, Some(2));
        assert!(problem[0][1].direction.is_none());
    }

    #[test]
    fn solves_arrow_without_given_number() {
        let problem = vec![vec![
            ArrowClue {
                direction: Some(4),
                count: None,
            },
            ArrowClue {
                direction: Some(3),
                count: None,
            },
        ]];
        let answer = solve_japanese_arrows(&problem).unwrap();
        assert_eq!(answer[0][0], Some(1));
    }

    #[test]
    fn allows_five_on_a_six_cell_ray() {
        let mut row = (0..6)
            .map(|x| ArrowClue {
                direction: Some(if x < 5 { 4 } else { 3 }),
                count: None,
            })
            .collect::<Vec<_>>();
        row[0] = ArrowClue {
            direction: Some(4),
            count: Some(5),
        };
        let answer = solve_japanese_arrows(&vec![row]).unwrap();
        assert_eq!(answer[0][0], Some(5));
    }

    #[test]
    fn round_trips_diagonal_arrow_without_given_number() {
        let problem = vec![vec![
            ArrowClue {
                direction: Some(8),
                count: None,
            },
            ArrowClue {
                direction: Some(3),
                count: None,
            },
        ]];
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn rejects_a_problem_with_a_missing_arrow() {
        let problem = vec![vec![
            ArrowClue {
                direction: Some(4),
                count: Some(1),
            },
            ArrowClue {
                direction: None,
                count: Some(1),
            },
        ]];
        assert_eq!(solve_japanese_arrows(&problem), None);
    }

    #[test]
    fn validates_the_four_by_four_example() {
        // The small example on the rules page is a completed 4x4 board.
        let numbers = [[3, 1, 2, 2], [2, 1, 1, 2], [1, 1, 2, 1], [3, 1, 2, 3]];
        let directions = [[8, 8, 3, 7], [8, 2, 2, 7], [2, 1, 1, 1], [1, 1, 3, 3]];
        let problem = (0..4)
            .map(|y| {
                (0..4)
                    .map(|x| ArrowClue {
                        direction: Some(directions[y][x]),
                        count: Some(numbers[y][x]),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let answer = solve_japanese_arrows(&problem).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(answer[y][x], Some(numbers[y][x]));
            }
        }
    }
}
