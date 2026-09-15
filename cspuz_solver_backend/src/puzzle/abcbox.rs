use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::Uniqueness;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Clue {
    letter: Option<usize>,
    length: Option<usize>,
}

// Parse exactly `count` outside-cell slots.  Empty slots (`x`) are retained
// as `None` so that the fixed-width top/left regions remain aligned; callers
// filter them out when constructing a line's actual clue list.
// Values greater than nine use pzpr's customary `-XX` hexadecimal form.
fn parse_clues(data: &[u8], count: usize) -> (Vec<Option<Clue>>, usize) {
    let mut slots = Vec::new();
    let mut pos = 0;
    for _ in 0..count {
        if pos >= data.len() {
            break;
        }
        let c = data[pos];
        pos += 1;
        match c {
            b'A' | b'a' => slots.push(Some(Clue {
                letter: Some(1),
                length: None,
            })),
            b'B' | b'b' => slots.push(Some(Clue {
                letter: Some(2),
                length: None,
            })),
            b'C' | b'c' => slots.push(Some(Clue {
                letter: Some(3),
                length: None,
            })),
            b'.' | b'?' => slots.push(Some(Clue::default())),
            b'1'..=b'9' => slots.push(Some(Clue {
                letter: None,
                length: Some((c - b'0') as usize),
            })),
            b'-' | b'+' | b'=' | b'@' | b'%' | b'*' | b'$' => {
                let digits = match c {
                    b'-' => 2,
                    b'+' => 3,
                    b'=' | b'@' | b'%' => 3,
                    b'*' => 4,
                    b'$' => 5,
                    _ => unreachable!(),
                };
                let mut clue = None;
                if pos + digits > data.len() {
                    slots.push(None);
                    continue;
                }
                if let Ok(text) = std::str::from_utf8(&data[pos..pos + digits]) {
                    if let Ok(length) = usize::from_str_radix(text, 16) {
                        let offset = match c {
                            b'=' => 4096,
                            b'@' | b'%' => 8192,
                            b'*' => 12240,
                            b'$' => 77776,
                            _ => 0,
                        };
                        if length + offset > 0 {
                            clue = Some(Clue {
                                letter: None,
                                length: Some(length + offset),
                            });
                        }
                    }
                }
                slots.push(clue);
                pos += digits;
            }
            _ => slots.push(None), // x is an unused outside-cell slot
        }
    }
    (slots, pos)
}

fn line_possible(line: &[usize], clues: &[Clue], complete: bool) -> bool {
    if clues.is_empty() {
        return true;
    }

    let mut groups: Vec<(usize, usize)> = Vec::new();
    for &value in line {
        if groups.last().map(|group| group.0) != Some(value) {
            groups.push((value, 1));
        } else if let Some(group) = groups.last_mut() {
            group.1 += 1;
        }
    }
    if groups.len() > clues.len() || (complete && groups.len() != clues.len()) {
        return false;
    }
    for (index, &(value, length)) in groups.iter().enumerate() {
        let clue = clues[index];
        if let Some(letter) = clue.letter {
            if letter != value {
                return false;
            }
        }
        if let Some(expected) = clue.length {
            let group_is_closed = complete || index + 1 < groups.len();
            if length > expected || (group_is_closed && length != expected) {
                return false;
            }
        }
    }
    true
}

fn generate_rows(width: usize, clues: &[Clue]) -> Vec<Vec<usize>> {
    fn visit(
        pos: usize,
        width: usize,
        clues: &[Clue],
        row: &mut Vec<usize>,
        result: &mut Vec<Vec<usize>>,
    ) {
        if pos == width {
            if line_possible(row, clues, true) {
                result.push(row.clone());
            }
            return;
        }
        for value in 1..=3 {
            row.push(value);
            if line_possible(row, clues, false) {
                visit(pos + 1, width, clues, row, result);
            }
            row.pop();
        }
    }

    let mut result = Vec::new();
    visit(0, width, clues, &mut Vec::new(), &mut result);
    result
}

fn solve_grid(
    rows: &[Vec<Vec<usize>>],
    vertical: &[Vec<Clue>],
    height: usize,
    width: usize,
) -> Option<Vec<Vec<usize>>> {
    fn search(
        y: usize,
        rows: &[Vec<Vec<usize>>],
        vertical: &[Vec<Clue>],
        height: usize,
        width: usize,
        grid: &mut Vec<Vec<usize>>,
    ) -> bool {
        if y == height {
            return (0..width).all(|x| {
                let column: Vec<_> = grid.iter().map(|row| row[x]).collect();
                line_possible(&column, &vertical[x], true)
            });
        }

        for row in &rows[y] {
            grid.push(row.clone());
            let valid = (0..width).all(|x| {
                let column: Vec<_> = grid.iter().map(|r| r[x]).collect();
                line_possible(&column, &vertical[x], false)
            });
            if valid && search(y + 1, rows, vertical, height, width, grid) {
                return true;
            }
            grid.pop();
        }
        false
    }

    let mut grid = Vec::new();
    search(0, rows, vertical, height, width, &mut grid).then_some(grid)
}

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let payload = url.split('?').nth(1).ok_or("invalid url")?;
    let parts: Vec<_> = payload.split('/').collect();
    if parts.len() < 3 || parts[0] != "abcbox" {
        return Err("invalid url");
    }
    let width: usize = parts[1].parse().map_err(|_| "invalid url")?;
    let height: usize = parts[2].parse().map_err(|_| "invalid url")?;
    if width == 0 || height == 0 {
        return Err("invalid url");
    }

    let ext = parts.get(4).copied().unwrap_or("").as_bytes();
    let (vertical_slots, used) = parse_clues(ext, width * height);
    let (horizontal_slots, _) = parse_clues(&ext[used..], width * height);
    let mut horizontal: Vec<Vec<Clue>> = Vec::with_capacity(height);
    let mut vertical: Vec<Vec<Clue>> = Vec::with_capacity(width);
    for y in 0..height {
        horizontal.push(
            horizontal_slots
                .iter()
                .skip(y * width)
                .take(width)
                .filter_map(|clue| *clue)
                .collect(),
        );
    }
    for x in 0..width {
        vertical.push(
            vertical_slots
                .iter()
                .skip(x * height)
                .take(height)
                .filter_map(|clue| *clue)
                .collect(),
        );
    }

    let row_candidates: Vec<_> = horizontal
        .iter()
        .map(|clues| generate_rows(width, clues))
        .collect();
    if row_candidates.iter().any(|rows| rows.is_empty()) {
        return Err("no answer");
    }
    let answer = solve_grid(&row_candidates, &vertical, height, width).ok_or("no answer")?;

    let mut board = Board::new(BoardKind::Grid, height, width, Uniqueness::NotApplicable);
    for (y, row) in answer.iter().enumerate() {
        for (x, &value) in row.iter().enumerate() {
            let letter = match value {
                1 => "A",
                2 => "B",
                3 => "C",
                _ => return Err("invalid answer"),
            };
            board.push(Item::cell(y, x, "green", ItemKind::Text(letter)));
        }
    }
    Ok(board)
}

#[cfg(test)]
mod tests {
    use super::solve;

    #[test]
    fn solves_letter_group_clues() {
        // A B / B A, with two letter clues on every line.
        let board = solve("https://pzv.jp/p.html?abcbox/2/2/3/ABBAABBA").unwrap();
        let json = board.to_json();
        assert!(json.contains("\"data\":\"A\""));
        assert!(json.contains("\"data\":\"B\""));
    }

    #[test]
    fn accepts_question_and_empty_clue_slots() {
        let board = solve("https://pzv.jp/p.html?abcbox/2/2/3/.xxxxxxx").unwrap();
        assert!(board.to_json().contains("\"height\":2"));
    }

    #[test]
    fn solves_numeric_and_mixed_group_clues() {
        // A A / B B.  Each column has two groups of length one; the row
        // clues mix a numeric length and a letter clue.
        let board = solve("https://pzv.jp/p.html?abcbox/2/2/3/11112xBx").unwrap();
        let json = board.to_json();
        assert!(json.contains("\"data\":\"A\""));
        assert!(json.contains("\"data\":\"B\""));
    }

    #[test]
    fn solves_two_digit_numeric_group_clues() {
        // A ten-cell row constrained by the hexadecimal -0a length form.
        let board = solve("https://pzv.jp/p.html?abcbox/10/1/3/1111111111-0axxxxxxxxx").unwrap();
        assert_eq!(board.to_json().matches("\"data\":\"A\"").count(), 10);
    }
}
