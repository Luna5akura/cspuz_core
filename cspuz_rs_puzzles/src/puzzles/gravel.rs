use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{from_base16, from_base36, to_base36};
use cspuz_rs::solver::{any, bool_constant, count_true, BoolExpr, Solver};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Circle {
    White,
    Black,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clue {
    pub circle: Option<Circle>,
    pub number: Option<i32>,
    pub empty: bool,
}

impl Clue {
    pub fn none() -> Clue {
        Clue {
            circle: None,
            number: None,
            empty: false,
        }
    }
}

pub type Problem = Vec<Vec<Clue>>;

#[derive(Debug, PartialEq, Eq)]
pub struct GravelAnswer {
    pub is_black: Vec<Vec<Option<bool>>>,
    pub borders: graph::BoolInnerGridEdgesIrrefutableFacts,
    pub is_unique: bool,
}

#[derive(Clone)]
struct Square {
    y: usize,
    x: usize,
    size: usize,
}

pub fn solve_gravel(problem: &Problem) -> Option<GravelAnswer> {
    let (h, w) = util::infer_shape(problem);
    if h == 0 || w == 0 {
        return None;
    }

    let valid = problem
        .iter()
        .map(|row| row.iter().map(|clue| !clue.empty).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let mut squares = vec![];
    let mut covering = vec![vec![Vec::<usize>::new(); w]; h];
    for y in 0..h {
        for x in 0..w {
            for size in 1..=((h - y).min(w - x)) {
                let mut ok = true;
                'cells: for yy in y..(y + size) {
                    for xx in x..(x + size) {
                        if !valid[yy][xx] {
                            ok = false;
                            break 'cells;
                        }
                    }
                }
                if ok {
                    let id = squares.len();
                    squares.push(Square { y, x, size });
                    for yy in y..(y + size) {
                        for xx in x..(x + size) {
                            covering[yy][xx].push(id);
                        }
                    }
                }
            }
        }
    }

    let mut solver = Solver::new();
    let is_black = &solver.bool_var_2d((h, w));
    let square_active = &solver.bool_var_1d(squares.len());
    let white_border = graph::BoolInnerGridEdges::new(&mut solver, (h, w));

    solver.add_answer_key_bool(is_black);
    solver.add_answer_key_bool(&white_border.horizontal);
    solver.add_answer_key_bool(&white_border.vertical);

    for y in 0..h {
        for x in 0..w {
            if !valid[y][x] {
                solver.add_expr(!is_black.at((y, x)));
                continue;
            }

            let cover_exprs = covering[y][x]
                .iter()
                .map(|&id| square_active.at(id))
                .collect::<Vec<_>>();
            solver.add_expr(count_true(cover_exprs).eq(is_black.at((y, x)).ite(0, 1)));

            match problem[y][x].circle {
                Some(Circle::White) => solver.add_expr(!is_black.at((y, x))),
                Some(Circle::Black) => solver.add_expr(is_black.at((y, x))),
                None => {}
            }

            if let Some(n) = problem[y][x].number {
                if n > 0 {
                    let cover_with_size = covering[y][x]
                        .iter()
                        .copied()
                        .filter(|&id| squares[id].size == n as usize)
                        .map(|id| square_active.at(id))
                        .collect::<Vec<_>>();
                    solver.add_expr((!is_black.at((y, x))).imp(count_true(cover_with_size).eq(1)));
                }
            }
        }
    }

    for id in 0..squares.len() {
        let sq = &squares[id];
        let active = square_active.at(id);

        let bottom_supported = if sq.y + sq.size >= h {
            bool_constant(true)
        } else {
            let yy = sq.y + sq.size;
            let mut supports = vec![];
            for xx in sq.x..(sq.x + sq.size) {
                if !valid[yy][xx] {
                    supports.push(bool_constant(true));
                } else {
                    supports.push(!is_black.at((yy, xx)));
                }
            }
            any(supports)
        };
        solver.add_expr(active.clone().imp(bottom_supported));

        let right_supported = if sq.x + sq.size >= w {
            bool_constant(true)
        } else {
            let xx = sq.x + sq.size;
            let mut supports = vec![];
            for yy in sq.y..(sq.y + sq.size) {
                if !valid[yy][xx] {
                    supports.push(bool_constant(true));
                } else {
                    supports.push(!is_black.at((yy, xx)));
                }
            }
            any(supports)
        };
        solver.add_expr(active.imp(right_supported));
    }

    for y in 0..h {
        for x in 0..w {
            if x + 1 < w {
                constrain_white_border(
                    &mut solver,
                    white_border.vertical.at((y, x)).expr(),
                    is_black,
                    &valid,
                    &covering,
                    square_active,
                    &squares,
                    (y, x),
                    (y, x + 1),
                );
            }
            if y + 1 < h {
                constrain_white_border(
                    &mut solver,
                    white_border.horizontal.at((y, x)).expr(),
                    is_black,
                    &valid,
                    &covering,
                    square_active,
                    &squares,
                    (y, x),
                    (y + 1, x),
                );
            }
        }
    }

    let black_border = graph::BoolInnerGridEdges::new(&mut solver, (h, w));
    for y in 0..h {
        for x in 0..w {
            if x + 1 < w {
                solver.add_expr(
                    black_border
                        .vertical
                        .at((y, x))
                        .iff(!(is_black.at((y, x)) & is_black.at((y, x + 1)))),
                );
            }
            if y + 1 < h {
                solver.add_expr(
                    black_border
                        .horizontal
                        .at((y, x))
                        .iff(!(is_black.at((y, x)) & is_black.at((y + 1, x)))),
                );
            }
        }
    }

    let black_size = &solver.int_var_2d((h, w), 1, (h * w) as i32);
    for y in 0..h {
        for x in 0..w {
            if !valid[y][x] {
                solver.add_expr(black_size.at((y, x)).eq(1));
            } else {
                solver.add_expr((!is_black.at((y, x))).imp(black_size.at((y, x)).eq(1)));
                if let Some(n) = problem[y][x].number {
                    if n > 0 {
                        solver.add_expr(is_black.at((y, x)).imp(black_size.at((y, x)).eq(n)));
                    }
                }
            }
        }
    }
    graph::graph_division_2d(&mut solver, black_size, &black_border);

    let facts = solver.irrefutable_facts()?;
    let is_black_answer = facts.get(is_black);
    let borders_answer = facts.get(&white_border);
    let is_unique = is_black_answer.iter().flatten().all(Option::is_some)
        && borders_answer
            .horizontal
            .iter()
            .flatten()
            .all(Option::is_some)
        && borders_answer
            .vertical
            .iter()
            .flatten()
            .all(Option::is_some);

    Some(GravelAnswer {
        is_black: is_black_answer,
        borders: borders_answer,
        is_unique,
    })
}

fn constrain_white_border(
    solver: &mut Solver,
    border: BoolExpr,
    is_black: &cspuz_rs::solver::BoolVarArray2D,
    valid: &[Vec<bool>],
    covering: &[Vec<Vec<usize>>],
    square_active: &cspuz_rs::solver::BoolVarArray1D,
    squares: &[Square],
    c1: (usize, usize),
    c2: (usize, usize),
) {
    let (y1, x1) = c1;
    let (y2, x2) = c2;
    if !valid[y1][x1] || !valid[y2][x2] {
        solver.add_expr(!border);
        return;
    }

    let both_white = !is_black.at(c1) & !is_black.at(c2);
    let mut same_square = vec![];
    for &id1 in &covering[y1][x1] {
        for &id2 in &covering[y2][x2] {
            if id1 == id2 {
                same_square.push(square_active.at(id1).expr());
            } else if squares[id1].size == squares[id2].size {
                solver.add_expr(!(square_active.at(id1) & square_active.at(id2)));
            }
        }
    }
    solver.add_expr(
        border.iff((is_black.at(c1) ^ is_black.at(c2)) | (both_white & !any_or_false(same_square))),
    );
}

fn any_or_false(exprs: Vec<BoolExpr>) -> BoolExpr {
    if exprs.is_empty() {
        bool_constant(false)
    } else {
        any(exprs)
    }
}

fn parse_gravel_url(url: &str) -> Option<(usize, usize, &str)> {
    let serialized = url.split('?').last().unwrap_or(url);
    let mut parts = serialized.split('/');
    let kind = parts.next()?;
    if kind != "gravel" {
        return None;
    }
    let width = parts.next()?.parse::<usize>().ok()?;
    let height = parts.next()?.parse::<usize>().ok()?;
    let data = parts.next().unwrap_or("");
    Some((height, width, data))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let (h, w, data) = parse_gravel_url(url)?;
    let mut problem = vec![vec![Clue::none(); w]; h];
    let n_cells = h * w;
    let bytes = data.as_bytes();
    let mut pos = 0;

    for base in (0..n_cells).step_by(3) {
        let val = from_base36(*bytes.get(pos)?).filter(|&v| v < 27)?;
        pos += 1;
        for offset in 0..3 {
            let idx = base + offset;
            if idx >= n_cells {
                break;
            }
            let div = [9, 3, 1][offset];
            let ques = (val / div) % 3;
            let cell = &mut problem[idx / w][idx % w];
            cell.circle = match ques {
                1 => Some(Circle::White),
                2 => Some(Circle::Black),
                _ => None,
            };
        }
    }

    let mut idx = 0;
    while idx < n_cells && pos < bytes.len() {
        if let Some((number, len)) = read_number16(bytes, pos) {
            problem[idx / w][idx % w].number = Some(number);
            pos += len;
            idx += 1;
        } else if let Some(skip) = skip_count(bytes[pos]) {
            idx += skip;
            pos += 1;
        } else {
            break;
        }
    }

    idx = 0;
    while idx < n_cells && pos < bytes.len() {
        let val = from_base36(bytes[pos]).filter(|&v| v < 32)?;
        pos += 1;
        for offset in 0..5 {
            if idx >= n_cells {
                break;
            }
            if (val & (1 << (4 - offset))) != 0 {
                let cell = &mut problem[idx / w][idx % w];
                cell.empty = true;
                cell.circle = None;
                cell.number = None;
            }
            idx += 1;
        }
    }

    Some(problem)
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = util::infer_shape(problem);
    let mut body = vec![];
    let n_cells = h * w;
    for base in (0..n_cells).step_by(3) {
        let mut val = 0;
        for offset in 0..3 {
            let idx = base + offset;
            if idx >= n_cells {
                break;
            }
            let clue = problem[idx / w][idx % w];
            let ques = if clue.empty {
                0
            } else {
                match clue.circle {
                    Some(Circle::White) => 1,
                    Some(Circle::Black) => 2,
                    None => 0,
                }
            };
            val += ques * [9, 3, 1][offset];
        }
        body.push(to_base36(val));
    }

    body.extend(encode_number16(problem));
    body.extend(encode_empty(problem));
    Some(format!(
        "https://puzz.link/p?gravel/{}/{}/{}",
        w,
        h,
        String::from_utf8(body).ok()?
    ))
}

fn read_number16(input: &[u8], pos: usize) -> Option<(i32, usize)> {
    let c = *input.get(pos)?;
    if let Some(n) = from_base16(c) {
        return Some((n, 1));
    }
    match c {
        b'-' => Some((read_fixed_hex(input, pos + 1, 2)?, 3)),
        b'+' => Some((read_fixed_hex(input, pos + 1, 3)?, 4)),
        b'=' => Some((read_fixed_hex(input, pos + 1, 3)? + 4096, 4)),
        b'@' => Some((read_fixed_hex(input, pos + 1, 3)? + 8192, 4)),
        b'*' => Some((read_fixed_hex(input, pos + 1, 4)? + 12240, 5)),
        b'$' => Some((read_fixed_hex(input, pos + 1, 5)? + 77776, 6)),
        b'.' => Some((-2, 1)),
        _ => None,
    }
}

fn read_fixed_hex(input: &[u8], pos: usize, len: usize) -> Option<i32> {
    let mut ret = 0;
    for i in 0..len {
        ret = ret * 16 + from_base16(*input.get(pos + i)?)?;
    }
    Some(ret)
}

fn skip_count(c: u8) -> Option<usize> {
    let val = from_base36(c)?;
    if (16..=35).contains(&val) {
        Some((val - 15) as usize)
    } else {
        None
    }
}

fn encode_number16(problem: &Problem) -> Vec<u8> {
    let mut ret = vec![];
    let mut skip = 0;
    for row in problem {
        for clue in row {
            let token = if clue.empty {
                None
            } else {
                clue.number.map(write_number16)
            };
            if let Some(token) = token {
                if skip > 0 {
                    ret.push(to_base36(skip as i32 + 15));
                    skip = 0;
                }
                ret.extend(token);
            } else {
                skip += 1;
                if skip == 20 {
                    ret.push(to_base36(skip + 15));
                    skip = 0;
                }
            }
        }
    }
    if skip > 0 {
        ret.push(to_base36(skip + 15));
    }
    ret
}

fn write_number16(n: i32) -> Vec<u8> {
    if n == -2 {
        vec![b'.']
    } else if (0..16).contains(&n) {
        vec![cspuz_rs::serializer::to_base16(n)]
    } else if n < 256 {
        format!("-{:02x}", n).into_bytes()
    } else if n < 4096 {
        format!("+{:03x}", n).into_bytes()
    } else if n < 8192 {
        format!("={:03x}", n - 4096).into_bytes()
    } else if n < 12240 {
        format!("@{:03x}", n - 8192).into_bytes()
    } else if n < 77776 {
        format!("*{:04x}", n - 12240).into_bytes()
    } else {
        format!("${:05x}", n - 77776).into_bytes()
    }
}

fn encode_empty(problem: &Problem) -> Vec<u8> {
    let mut ret = vec![];
    let mut found = false;
    let flat = problem
        .iter()
        .flat_map(|row| row.iter())
        .collect::<Vec<_>>();
    for base in (0..flat.len()).step_by(5) {
        let mut val = 0;
        for offset in 0..5 {
            let idx = base + offset;
            if idx >= flat.len() {
                break;
            }
            if flat[idx].empty {
                found = true;
                val |= 1 << (4 - offset);
            }
        }
        ret.push(to_base36(val));
    }
    if found {
        ret
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gravel_serializer() {
        let url = "https://puzz.link/p?gravel/7/7/00000000000000000zzoos200431su";
        let problem = deserialize_problem(url).unwrap();
        assert_eq!(problem.len(), 7);
        assert_eq!(problem[0].len(), 7);
        assert_eq!(serialize_problem(&problem), Some(url.to_string()));
    }

    #[test]
    fn test_gravel_white_one_cell() {
        let problem = vec![vec![Clue {
            circle: Some(Circle::White),
            number: Some(1),
            empty: false,
        }]];
        let ans = solve_gravel(&problem).unwrap();
        assert_eq!(ans.is_black, vec![vec![Some(false)]]);
        assert!(ans.is_unique);
    }

    #[test]
    fn test_gravel_black_one_cell() {
        let problem = vec![vec![Clue {
            circle: Some(Circle::Black),
            number: Some(1),
            empty: false,
        }]];
        let ans = solve_gravel(&problem).unwrap();
        assert_eq!(ans.is_black, vec![vec![Some(true)]]);
        assert!(ans.is_unique);
    }
}
