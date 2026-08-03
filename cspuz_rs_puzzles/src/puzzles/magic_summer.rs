use cspuz_rs::serializer::strip_prefix;
use cspuz_rs::solver::{IntVarArray1D, Solver};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub count: usize,
    pub side_clues: Vec<Option<i32>>,
    pub cell_clues: Vec<Vec<Option<i32>>>,
}

fn max_number(length: usize) -> Option<i32> {
    let mut value = 0i32;
    for _ in 0..length {
        value = value.checked_mul(10)?.checked_add(9)?;
    }
    Some(value)
}

fn add_sum_constraint(solver: &mut Solver, line: IntVarArray1D, clue: Option<i32>, max_sum: i32) {
    let Some(clue) = clue.filter(|clue| *clue >= 0) else {
        return;
    };
    let len = line.len();
    let upper = clue.min(max_sum);
    let partial = &solver.int_var_1d(len, 0, upper);
    let total = &solver.int_var_1d(len + 1, 0, upper);
    solver.add_expr(total.at(0).eq(0));

    for i in 0..len {
        let concatenated = if i == 0 {
            line.at(i).expr()
        } else {
            (0..10).fold(line.at(i).expr(), |acc, _| acc + partial.at(i - 1).expr())
        };
        solver.add_expr(partial.at(i).eq(line.at(i).eq(0).ite(0, concatenated)));

        let added = if i == 0 {
            line.at(i).expr()
        } else {
            line.at(i - 1)
                .eq(0)
                .ite(line.at(i).expr(), partial.at(i) - partial.at(i - 1))
        };
        solver.add_expr(
            total.at(i + 1).eq(line
                .at(i)
                .eq(0)
                .ite(total.at(i).expr(), total.at(i).expr() + added)),
        );
    }

    solver.add_expr(total.at(len).eq(clue));
}

pub fn solve_magic_summer(problem: &Problem) -> Option<Vec<Vec<Option<i32>>>> {
    let height = problem.cell_clues.len();
    if height == 0 {
        return None;
    }
    let width = problem.cell_clues[0].len();
    if width == 0 || problem.cell_clues.iter().any(|row| row.len() != width) {
        return None;
    }
    if problem.count == 0 || problem.count > height || problem.count > width || problem.count > 9 {
        return None;
    }
    if problem.side_clues.len() != 2 * width + 2 * height {
        return None;
    }

    let max_row_sum = max_number(width)?;
    let max_col_sum = max_number(height)?;
    let count = problem.count as i32;
    let mut solver = Solver::new();
    let num = &solver.int_var_2d((height, width), 0, count);
    solver.add_answer_key_int(num);

    for y in 0..height {
        for n in 1..=count {
            solver.add_expr(num.slice_fixed_y((y, ..)).eq(n).count_true().eq(1));
        }
    }
    for x in 0..width {
        for n in 1..=count {
            solver.add_expr(num.slice_fixed_x((.., x)).eq(n).count_true().eq(1));
        }
    }

    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = problem.cell_clues[y][x] {
                if clue == -2 {
                    solver.add_expr(num.at((y, x)).eq(0));
                } else if clue < 1 || clue > count {
                    return None;
                } else {
                    solver.add_expr(num.at((y, x)).eq(clue));
                }
            }
        }
    }

    let mut clue_index = 0;
    for x in 0..width {
        let line = IntVarArray1D::new((0..height).map(|y| num.at((y, x))));
        add_sum_constraint(
            &mut solver,
            line,
            problem.side_clues[clue_index],
            max_col_sum,
        );
        clue_index += 1;
    }
    for x in 0..width {
        let line = IntVarArray1D::new((0..height).rev().map(|y| num.at((y, x))));
        add_sum_constraint(
            &mut solver,
            line,
            problem.side_clues[clue_index],
            max_col_sum,
        );
        clue_index += 1;
    }
    for y in 0..height {
        let line = IntVarArray1D::new((0..width).map(|x| num.at((y, x))));
        add_sum_constraint(
            &mut solver,
            line,
            problem.side_clues[clue_index],
            max_row_sum,
        );
        clue_index += 1;
    }
    for y in 0..height {
        let line = IntVarArray1D::new((0..width).rev().map(|x| num.at((y, x))));
        add_sum_constraint(
            &mut solver,
            line,
            problem.side_clues[clue_index],
            max_row_sum,
        );
        clue_index += 1;
    }

    solver.irrefutable_facts().map(|f| f.get(num))
}

fn from_base16(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'f' => Some((c - b'a' + 10) as i32),
        b'A'..=b'F' => Some((c - b'A' + 10) as i32),
        _ => None,
    }
}

fn from_base36(c: u8) -> Option<usize> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as usize),
        b'a'..=b'z' => Some((c - b'a' + 10) as usize),
        b'A'..=b'Z' => Some((c - b'A' + 10) as usize),
        _ => None,
    }
}

fn to_base36(n: usize) -> Option<char> {
    match n {
        0..=9 => Some((b'0' + n as u8) as char),
        10..=35 => Some((b'a' + (n - 10) as u8) as char),
        _ => None,
    }
}

fn read_number16(input: &[u8], pos: usize) -> Option<(i32, usize)> {
    let c = *input.get(pos)?;
    if let Some(n) = from_base16(c) {
        Some((n, 1))
    } else if c == b'-' {
        Some((
            (from_base16(*input.get(pos + 1)?)? << 4) | from_base16(*input.get(pos + 2)?)?,
            3,
        ))
    } else if c == b'+' {
        Some((
            (from_base16(*input.get(pos + 1)?)? << 8)
                | (from_base16(*input.get(pos + 2)?)? << 4)
                | from_base16(*input.get(pos + 3)?)?,
            4,
        ))
    } else if c == b'=' {
        Some((
            4096 + (from_base16(*input.get(pos + 1)?)? << 8)
                + (from_base16(*input.get(pos + 2)?)? << 4)
                + from_base16(*input.get(pos + 3)?)?,
            4,
        ))
    } else if c == b'%' || c == b'@' {
        Some((
            8192 + (from_base16(*input.get(pos + 1)?)? << 8)
                + (from_base16(*input.get(pos + 2)?)? << 4)
                + from_base16(*input.get(pos + 3)?)?,
            4,
        ))
    } else if c == b'*' {
        Some((
            12240
                + (from_base16(*input.get(pos + 1)?)? << 12)
                + (from_base16(*input.get(pos + 2)?)? << 8)
                + (from_base16(*input.get(pos + 3)?)? << 4)
                + from_base16(*input.get(pos + 4)?)?,
            5,
        ))
    } else if c == b'$' {
        Some((
            77776
                + (from_base16(*input.get(pos + 1)?)? << 16)
                + (from_base16(*input.get(pos + 2)?)? << 12)
                + (from_base16(*input.get(pos + 3)?)? << 8)
                + (from_base16(*input.get(pos + 4)?)? << 4)
                + from_base16(*input.get(pos + 5)?)?,
            6,
        ))
    } else if c == b'.' {
        Some((-2, 1))
    } else {
        None
    }
}

fn decode_number16_sequence(input: &str, len: usize) -> Option<(usize, Vec<Option<i32>>)> {
    let bytes = input.as_bytes();
    let mut ret = vec![None; len];
    let mut cursor = 0usize;
    let mut pos = 0usize;

    while pos < bytes.len() && cursor < len {
        if let Some((n, consumed)) = read_number16(bytes, pos) {
            ret[cursor] = Some(n);
            pos += consumed;
            cursor += 1;
        } else {
            let c = bytes[pos];
            if (b'g'..=b'z').contains(&c) {
                cursor = (cursor + from_base36(c)? - 15).min(len);
            }
            pos += 1;
        }
    }

    Some((pos, ret))
}

fn write_number16(n: i32) -> Option<String> {
    if n == -2 {
        Some(".".to_string())
    } else if (0..16).contains(&n) {
        Some(format!("{:x}", n))
    } else if (16..256).contains(&n) {
        Some(format!("-{:02x}", n))
    } else if (256..4096).contains(&n) {
        Some(format!("+{:03x}", n))
    } else if (4096..8192).contains(&n) {
        Some(format!("={:03x}", n - 4096))
    } else if (8192..12240).contains(&n) {
        Some(format!("@{:03x}", n - 8192))
    } else if (12240..77776).contains(&n) {
        Some(format!("*{:04x}", n - 12240))
    } else if n >= 77776 {
        Some(format!("${:05x}", n - 77776))
    } else {
        None
    }
}

fn encode_number16_sequence(values: &[Option<i32>]) -> Option<String> {
    let mut ret = String::new();
    let mut blanks = 0usize;

    for &value in values {
        let encoded = match value {
            Some(n) => write_number16(n)?,
            None => {
                blanks += 1;
                String::new()
            }
        };

        if blanks == 0 {
            ret.push_str(&encoded);
        } else if !encoded.is_empty() || blanks == 20 {
            ret.push(to_base36(15 + blanks)?);
            ret.push_str(&encoded);
            blanks = 0;
        }
    }
    if blanks > 0 {
        ret.push(to_base36(15 + blanks)?);
    }

    Some(ret)
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let height = problem.cell_clues.len();
    if height == 0 {
        return None;
    }
    let width = problem.cell_clues[0].len();
    if width == 0 || problem.cell_clues.iter().any(|row| row.len() != width) {
        return None;
    }
    if problem.side_clues.len() != 2 * width + 2 * height {
        return None;
    }

    let mut url = format!(
        "https://puzz.link/p?magic-summer/{}/{}/{}/{}",
        width,
        height,
        problem.count,
        encode_number16_sequence(&problem.side_clues)?
    );

    if problem.cell_clues.iter().flatten().any(|x| x.is_some()) {
        let cells = problem
            .cell_clues
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        url.push_str(&encode_number16_sequence(&cells)?);
    }

    Some(url)
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let stripped = strip_prefix(url)?;
    let mut parts = stripped.splitn(5, '/');
    let kind = parts.next()?;
    if kind != "magic-summer" && kind != "magicsummer" {
        return None;
    }
    let width = parts.next()?.parse::<usize>().ok()?;
    let height = parts.next()?.parse::<usize>().ok()?;
    let count = parts.next()?.parse::<usize>().ok()?;
    let encoded = parts.next().unwrap_or("");

    if width == 0 || height == 0 {
        return None;
    }

    let side_len = 2 * width + 2 * height;
    let (consumed, side_clues) = decode_number16_sequence(encoded, side_len)?;
    let rest = encoded.get(consumed..)?;
    let (_, flat_cells) = decode_number16_sequence(rest, width * height)?;
    let mut cell_clues = vec![vec![None; width]; height];
    for y in 0..height {
        for x in 0..width {
            cell_clues[y][x] = flat_cells[y * width + x];
        }
    }

    Some(Problem {
        count,
        side_clues,
        cell_clues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        Problem {
            count: 2,
            side_clues: vec![
                Some(12),
                Some(3),
                Some(12),
                Some(21),
                Some(3),
                Some(21),
                Some(12),
                Some(3),
                Some(12),
                Some(21),
                Some(3),
                Some(21),
            ],
            cell_clues: vec![vec![None; 3]; 3],
        }
    }

    #[test]
    fn test_magic_summer_problem() {
        let ans = solve_magic_summer(&problem_for_tests()).unwrap();
        let expected = crate::util::tests::to_option_2d([[1, 2, 0], [2, 0, 1], [0, 1, 2]]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_magic_summer_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?magic-summer/3/3/2/c3c-153-15c3c-153-15";
        crate::util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
