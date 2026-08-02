use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine,
};
use cspuz_rs::serializer::strip_prefix;
use cspuz_rs::solver::{any, count_true, sum, Solver};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlovakSumsClue {
    pub sum: Option<i32>,
    pub count: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlovakSumsCell {
    Blocked,
    Clue(SlovakSumsClue),
}

pub type Problem = (Vec<i32>, Vec<Vec<Option<SlovakSumsCell>>>);

pub fn solve_slovak_sums(
    numbers: &[i32],
    clues: &[Vec<Option<SlovakSumsCell>>],
) -> Option<Vec<Vec<Option<i32>>>> {
    if numbers.is_empty() || clues.is_empty() || clues[0].is_empty() {
        return None;
    }

    let height = clues.len();
    let width = clues[0].len();
    if clues.iter().any(|row| row.len() != width) || numbers.iter().any(|&n| n <= 0) {
        return None;
    }

    let max_number = *numbers.iter().max().unwrap();
    let mut solver = Solver::new();
    let num = &solver.int_var_2d((height, width), 0, max_number);
    solver.add_answer_key_int(num);

    for y in 0..height {
        for x in 0..width {
            if clues[y][x].is_some() {
                solver.add_expr(num.at((y, x)).eq(0));
            } else {
                let mut candidates = vec![num.at((y, x)).eq(0)];
                candidates.extend(numbers.iter().map(|&n| num.at((y, x)).eq(n)));
                solver.add_expr(any(candidates));
            }
        }
    }

    for y in 0..height {
        for &n in numbers {
            solver.add_expr(num.slice_fixed_y((y, ..)).eq(n).count_true().eq(1));
        }
    }
    for x in 0..width {
        for &n in numbers {
            solver.add_expr(num.slice_fixed_x((.., x)).eq(n).count_true().eq(1));
        }
    }

    for y in 0..height {
        for x in 0..width {
            let Some(clue) = clues[y][x] else {
                continue;
            };
            let SlovakSumsCell::Clue(clue) = clue else {
                continue;
            };

            let mut neighbors = vec![];
            if y > 0 && clues[y - 1][x].is_none() {
                neighbors.push((y - 1, x));
            }
            if y + 1 < height && clues[y + 1][x].is_none() {
                neighbors.push((y + 1, x));
            }
            if x > 0 && clues[y][x - 1].is_none() {
                neighbors.push((y, x - 1));
            }
            if x + 1 < width && clues[y][x + 1].is_none() {
                neighbors.push((y, x + 1));
            }

            if neighbors.is_empty() {
                if clue.count.unwrap_or(0) != 0 || clue.sum.unwrap_or(0) != 0 {
                    return None;
                }
                continue;
            }

            let is_filled = neighbors
                .iter()
                .map(|&(ny, nx)| num.at((ny, nx)).ne(0))
                .collect::<Vec<_>>();
            let values = neighbors
                .iter()
                .map(|&(ny, nx)| num.at((ny, nx)))
                .collect::<Vec<_>>();
            if let Some(count) = clue.count {
                solver.add_expr(count_true(is_filled).eq(count));
            }
            if let Some(sum_value) = clue.sum {
                solver.add_expr(sum(values).eq(sum_value));
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(num))
}

fn decode_payload(encoded: &str) -> Option<json::JsonValue> {
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .or_else(|_| URL_SAFE.decode(encoded))
        .or_else(|_| STANDARD_NO_PAD.decode(encoded))
        .or_else(|_| STANDARD.decode(encoded))
        .ok()?;
    let decoded = std::str::from_utf8(&decoded).ok()?;
    json::parse(decoded).ok()
}

fn decode_number16(encoded: &str, length: usize) -> Option<Vec<i32>> {
    fn read_hex(encoded: &[u8], start: usize, length: usize) -> Option<i32> {
        let end = start.checked_add(length)?;
        let text = std::str::from_utf8(encoded.get(start..end)?).ok()?;
        i32::from_str_radix(text, 16).ok()
    }

    fn read_number16(encoded: &[u8], index: usize) -> Option<(i32, usize)> {
        let c = *encoded.get(index)?;
        match c {
            b'0'..=b'9' | b'a'..=b'f' => Some(((c as char).to_digit(16)? as i32, 1)),
            b'-' => Some((read_hex(encoded, index + 1, 2)?, 3)),
            b'+' => Some((read_hex(encoded, index + 1, 3)?, 4)),
            b'=' => Some((read_hex(encoded, index + 1, 3)? + 4096, 4)),
            b'%' | b'@' => Some((read_hex(encoded, index + 1, 3)? + 8192, 4)),
            b'*' => Some((read_hex(encoded, index + 1, 4)? + 12240, 5)),
            b'$' => Some((read_hex(encoded, index + 1, 5)? + 77776, 6)),
            b'.' => Some((-2, 1)),
            _ => None,
        }
    }

    let encoded = encoded.as_bytes();
    let mut values = vec![-1; length];
    let mut cell_index = 0;
    let mut index = 0;
    while index < encoded.len() && cell_index < length {
        if let Some((value, consumed)) = read_number16(encoded, index) {
            values[cell_index] = value;
            cell_index += 1;
            index += consumed;
        } else if encoded[index].is_ascii_lowercase() && (b'g'..=b'z').contains(&encoded[index]) {
            cell_index += (encoded[index] as usize) - ('g' as usize) + 1;
            index += 1;
        } else {
            index += 1;
        }
    }

    if cell_index == length {
        Some(values)
    } else {
        None
    }
}

fn deserialize_native_problem(
    width: usize,
    height: usize,
    number_count: usize,
    encoded: &str,
) -> Option<Problem> {
    if number_count == 0 {
        return None;
    }

    let values = decode_number16(encoded, width.checked_mul(height)?)?;
    let cells = values
        .chunks(width)
        .map(|row| {
            row.iter()
                .map(|&value| {
                    if value == -1 {
                        None
                    } else if value == -2 || value == 0 {
                        Some(SlovakSumsCell::Blocked)
                    } else if (1..=4).contains(&value) {
                        Some(SlovakSumsCell::Clue(SlovakSumsClue {
                            sum: None,
                            count: (value > 0).then_some(value),
                        }))
                    } else {
                        let sum = value / 5 - 1;
                        let count = value % 5;
                        Some(SlovakSumsCell::Clue(SlovakSumsClue {
                            sum: Some(sum),
                            count: (count > 0).then_some(count),
                        }))
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Some(((1..=number_count as i32).collect(), cells))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let serialized = strip_prefix(url)?;
    let mut parts = serialized.split('/');
    let kind = parts.next()?;

    let width = parts.next()?.parse::<usize>().ok()?;
    let height = parts.next()?.parse::<usize>().ok()?;
    if width == 0 || height == 0 {
        return None;
    }

    let remaining = parts.collect::<Vec<_>>();
    if remaining.is_empty() {
        return None;
    }

    if kind == "slovak" || kind == "slovak-sums" {
        if let Some(number_count) = remaining[0].parse::<usize>().ok() {
            let encoded = remaining[1..].join("/");
            if encoded.is_empty() {
                return None;
            }
            return deserialize_native_problem(width, height, number_count, &encoded);
        }
    }

    if kind != "slovak-sums" && kind != "slovaksums" {
        return None;
    }

    let encoded = remaining.join("/");
    if encoded.is_empty() {
        return None;
    }
    let payload = decode_payload(&encoded)?;

    let numbers_data = &payload["numbers"];
    if !numbers_data.is_array() {
        return None;
    }
    let numbers = numbers_data
        .members()
        .map(|value| value.as_i32())
        .collect::<Option<Vec<_>>>()?;
    if numbers.is_empty() || numbers.iter().any(|&n| n <= 0) {
        return None;
    }

    let cells = &payload["cells"];
    if !cells.is_array() || cells.len() != height {
        return None;
    }

    let mut clues = Vec::with_capacity(height);
    for row in cells.members() {
        if !row.is_array() || row.len() != width {
            return None;
        }

        let mut clue_row = Vec::with_capacity(width);
        for cell in row.members() {
            if cell.is_null() {
                clue_row.push(None);
                continue;
            }
            if !cell.is_object() {
                return None;
            }

            let sum = cell["sum"].as_i32()?;
            let count = cell["count"].as_i32()?;
            if sum < 0 || count < 0 {
                return None;
            }
            clue_row.push(Some(SlovakSumsCell::Clue(SlovakSumsClue {
                sum: Some(sum),
                count: Some(count),
            })));
        }
        clues.push(clue_row);
    }

    Some((numbers, clues))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        (
            vec![1, 2],
            vec![
                vec![
                    None,
                    None,
                    Some(SlovakSumsCell::Clue(SlovakSumsClue {
                        sum: Some(3),
                        count: Some(2),
                    })),
                ],
                vec![
                    None,
                    Some(SlovakSumsCell::Clue(SlovakSumsClue {
                        sum: Some(6),
                        count: Some(4),
                    })),
                    None,
                ],
                vec![
                    Some(SlovakSumsCell::Clue(SlovakSumsClue {
                        sum: Some(3),
                        count: Some(2),
                    })),
                    None,
                    None,
                ],
            ],
        )
    }

    #[test]
    fn test_slovak_sums() {
        let (numbers, clues) = problem_for_tests();
        let answer = solve_slovak_sums(&numbers, &clues).unwrap();
        assert_eq!(
            answer,
            vec![
                vec![None, None, Some(0)],
                vec![None, Some(0), None],
                vec![Some(0), None, None],
            ]
        );
        assert_eq!(answer[0][2], Some(0));
        assert_eq!(answer[1][1], Some(0));
        assert_eq!(answer[2][0], Some(0));
    }

    #[test]
    fn test_slovak_sums_rejects_inconsistent_clue() {
        let (numbers, mut clues) = problem_for_tests();
        clues[0][2] = Some(SlovakSumsCell::Clue(SlovakSumsClue {
            sum: Some(4),
            count: Some(2),
        }));
        assert!(solve_slovak_sums(&numbers, &clues).is_none());
    }

    #[test]
    fn test_slovak_sums_serializer() {
        let payload = r#"{"numbers":[1,2],"cells":[[null,null,{"sum":3,"count":2}],[null,{"sum":6,"count":4},null],[{"sum":3,"count":2},null,null]]}"#;
        let encoded = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        let url = format!("https://puzz.link/p?slovak-sums/3/3/{encoded}");
        assert_eq!(deserialize_problem(&url), Some(problem_for_tests()));
    }

    #[test]
    fn test_slovak_native_serializer() {
        let url = "https://puzz.link/p?slovak-sums/5/5/3/1n-28i-26g0m-1cg";
        let (numbers, cells) = deserialize_problem(url).unwrap();

        assert_eq!(numbers, vec![1, 2, 3]);
        assert_eq!(
            cells[0][0],
            Some(SlovakSumsCell::Clue(SlovakSumsClue {
                sum: None,
                count: Some(1),
            }))
        );
        assert_eq!(
            cells[1][4],
            Some(SlovakSumsCell::Clue(SlovakSumsClue {
                sum: Some(7),
                count: None,
            }))
        );
        assert_eq!(cells[3][0], Some(SlovakSumsCell::Blocked));
    }
}
