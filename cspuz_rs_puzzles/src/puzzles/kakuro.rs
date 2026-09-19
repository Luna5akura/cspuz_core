use crate::util;
use cspuz_rs::complex_constraints::sum_all_different;
use cspuz_rs::serializer::{
    problem_to_url_with_context, strip_prefix, url_to_problem, Choice, Combinator, Context,
    ContextBasedGrid, Dict, Optionalize, Size, Spaces, Tuple2, UnlimitedSeq,
};
use cspuz_rs::solver::{IntVar, IntVarArray1D, Solver};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KakuroClue {
    pub down: Option<i32>,
    pub right: Option<i32>,
}

pub fn solve_kakuro(clues: &[Vec<Option<KakuroClue>>]) -> Option<Vec<Vec<Option<i32>>>> {
    solve_kakuro_with_bars(clues, None)
}

fn add_consecutive_constraint(solver: &mut Solver, a: IntVar, b: IntVar, consecutive: bool) {
    let diff = a - b;
    if consecutive {
        solver.add_expr(diff.eq(1) | diff.eq(-1));
    } else {
        solver.add_expr(diff.ne(1) & diff.ne(-1));
    }
}

pub fn solve_kakuro_with_bars(
    clues: &[Vec<Option<KakuroClue>>],
    bars: Option<&[Vec<u8>]>,
) -> Option<Vec<Vec<Option<i32>>>> {
    let (h, w) = util::infer_shape(clues);

    let mut solver = Solver::new();
    let numbers = &solver.int_var_2d((h, w), 0, 9);
    solver.add_answer_key_int(numbers);

    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_some() {
                solver.add_expr(numbers.at((y, x)).eq(0));
            } else {
                solver.add_expr(numbers.at((y, x)).ne(0));
            }
        }
    }

    let mut dict = vec![vec![vec![]; 46]; 10];
    for b in 1i32..512 {
        let num = b.count_ones();
        let mut sum = 0;
        for i in 0..9 {
            if (b & (1 << i)) != 0 {
                sum += i + 1;
            }
        }
        dict[num as usize][sum as usize].push(b);
    }

    let mut add_constraints = |cells: IntVarArray1D, clue: Option<i32>| -> bool {
        if let Some(n) = clue {
            sum_all_different(&mut solver, cells, n, 1, 9, None)
        } else {
            // A blank (rather than zero) clue still denotes a run whose
            // digits must be distinct; only its sum is unspecified.
            solver.all_different(&cells);
            cells.len() <= 9
        }
    };

    for y in 0..h {
        for x in 0..w {
            if let Some(clue) = clues[y][x] {
                // down
                let mut y2 = y + 1;
                while y2 < h && clues[y2][x].is_none() {
                    y2 += 1;
                }
                if y2 - y >= 2 {
                    if !add_constraints(numbers.slice_fixed_x(((y + 1)..y2, x)), clue.down) {
                        return None;
                    }
                }

                // right
                let mut x2 = x + 1;
                while x2 < w && clues[y][x2].is_none() {
                    x2 += 1;
                }
                if x2 - x >= 2 {
                    if !add_constraints(numbers.slice_fixed_y((y, (x + 1)..x2)), clue.right) {
                        return None;
                    }
                }
            }
        }
    }

    if let Some(bars) = bars {
        if bars.len() != h - 1 || bars.iter().any(|row| row.len() != w - 1) {
            return None;
        }
        for y in 1..h {
            for x in 1..w {
                if clues[y][x].is_some() {
                    continue;
                }
                let cell_bars = bars[y - 1][x - 1];
                if x + 1 < w && clues[y][x + 1].is_none() {
                    add_consecutive_constraint(
                        &mut solver,
                        numbers.at((y, x)),
                        numbers.at((y, x + 1)),
                        cell_bars & 2 != 0,
                    );
                }
                if y + 1 < h && clues[y + 1][x].is_none() {
                    add_consecutive_constraint(
                        &mut solver,
                        numbers.at((y, x)),
                        numbers.at((y + 1, x)),
                        cell_bars & 1 != 0,
                    );
                }
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(numbers))
}

struct KakuroNumCombinator;

impl Combinator<Option<i32>> for KakuroNumCombinator {
    fn serialize(&self, _: &Context, input: &[Option<i32>]) -> Option<(usize, Vec<u8>)> {
        if input.is_empty() {
            return None;
        }
        let n = input[0];

        if n.is_none() {
            return Some((1, vec![b'-']));
        }

        let n = n.unwrap();
        let c = if (1..=9).contains(&n) {
            n as u8 + b'0'
        } else if (10..=19).contains(&n) {
            n as u8 - 10 + b'a'
        } else if (20..=45).contains(&n) {
            n as u8 - 20 + b'A'
        } else {
            return None;
        };

        Some((1, vec![c]))
    }

    fn deserialize(&self, _: &Context, input: &[u8]) -> Option<(usize, Vec<Option<i32>>)> {
        if input.is_empty() {
            return None;
        }
        let c = input[0];

        if c == b'-' {
            return Some((1, vec![None]));
        }

        let v = if (b'0'..=b'9').contains(&c) {
            c - b'0'
        } else if (b'a'..=b'j').contains(&c) {
            c - b'a' + 10
        } else if (b'A'..=b'Z').contains(&c) {
            c - b'A' + 20
        } else {
            return None;
        };

        Some((1, vec![Some(v as i32)]))
    }
}

pub type Problem = Vec<Vec<Option<KakuroClue>>>;

type IntermediateProblem = (
    Vec<Vec<Option<(Option<i32>, Option<i32>)>>>,
    Vec<Option<i32>>,
);

fn combinator() -> impl Combinator<IntermediateProblem> {
    Size::new(Tuple2::new(
        ContextBasedGrid::new(Choice::new(vec![
            Box::new(Optionalize::new(Tuple2::new(
                KakuroNumCombinator,
                KakuroNumCombinator,
            ))),
            Box::new(Dict::new(Some((None, None)), ".")),
            Box::new(Spaces::new(None, 'k')),
        ])),
        UnlimitedSeq::new(KakuroNumCombinator),
    ))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = util::infer_shape(problem);
    if !(h >= 2 && w >= 2) {
        return None;
    }

    let mut intermediate_grid = vec![vec![None; w - 1]; h - 1];
    for y in 0..(h - 1) {
        for x in 0..(w - 1) {
            intermediate_grid[y][x] = problem[y + 1][x + 1].map(|clue| (clue.down, clue.right));
        }
    }

    let mut rem_seq = vec![];
    for x in 1..w {
        if problem[1][x].is_none() {
            if problem[0][x].is_none() {
                return None;
            }
            rem_seq.push(problem[0][x].unwrap().down);
        }
    }
    for y in 1..h {
        if problem[y][1].is_none() {
            if problem[y][0].is_none() {
                return None;
            }
            rem_seq.push(problem[y][0].unwrap().right);
        }
    }

    problem_to_url_with_context(
        combinator(),
        "kakuro",
        (intermediate_grid, rem_seq),
        &Context::sized(h - 1, w - 1),
    )
}

type IntermediateGrid = Vec<Vec<Option<(Option<i32>, Option<i32>)>>>;

fn problem_from_intermediate(
    intermediate_grid: IntermediateGrid,
    rem_seq: Vec<Option<i32>>,
) -> Option<Problem> {
    let (h, w) = util::infer_shape(&intermediate_grid);
    let h = h + 1;
    let w = w + 1;

    let mut ret = vec![vec![None; w]; h];
    for y in 1..h {
        for x in 1..w {
            ret[y][x] =
                intermediate_grid[y - 1][x - 1].map(|(down, right)| KakuroClue { down, right });
        }
    }
    let mut idx = 0;
    ret[0][0] = Some(KakuroClue {
        down: None,
        right: None,
    });
    for x in 1..w {
        if ret[1][x].is_none() {
            if idx >= rem_seq.len() {
                return None;
            }
            ret[0][x] = Some(KakuroClue {
                down: rem_seq[idx],
                right: None,
            });
            idx += 1;
        } else {
            ret[0][x] = Some(KakuroClue {
                down: None,
                right: None,
            });
        }
    }
    for y in 1..h {
        if ret[y][1].is_none() {
            if idx >= rem_seq.len() {
                return None;
            }
            ret[y][0] = Some(KakuroClue {
                down: None,
                right: rem_seq[idx],
            });
            idx += 1;
        } else {
            ret[y][0] = Some(KakuroClue {
                down: None,
                right: None,
            });
        }
    }

    Some(ret)
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let (intermediate_grid, rem_seq) =
        url_to_problem(combinator(), &["kakuro", "consecutivekakuro"], url)?;
    problem_from_intermediate(intermediate_grid, rem_seq)
}

fn decode_kakuro_num(ca: u8) -> Option<i32> {
    match ca {
        b'0'..=b'9' => Some((ca - b'0') as i32),
        b'a'..=b'j' => Some((ca - b'a' + 10) as i32),
        b'A'..=b'Z' => Some((ca - b'A' + 20) as i32),
        _ => None,
    }
}

fn hex_digit(ca: u8) -> Option<i32> {
    match ca {
        b'0'..=b'9' => Some((ca - b'0') as i32),
        b'a'..=b'f' => Some((ca - b'a' + 10) as i32),
        b'A'..=b'F' => Some((ca - b'A' + 10) as i32),
        _ => None,
    }
}

/// Decode the pzprjs body of a Kakuro URL: the row-major cell stream, the
/// outside clue characters and, for Consecutive Kakuro, the trailing
/// white-bar segment.
///
/// `cols` and `rows` are the playable grid dimensions from the URL header.
/// The returned grid uses the serializer's intermediate representation:
/// `Some((down, right))` for a clue cell, `None` for a white cell.
fn decode_kakuro_body(
    body: &[u8],
    rows: usize,
    cols: usize,
) -> Option<(IntermediateGrid, Vec<Option<i32>>, &[u8])> {
    let total = rows.checked_mul(cols)?;
    let mut grid = vec![vec![None; cols]; rows];
    let mut pos = 0usize;
    let mut cell = 0usize;

    while pos < body.len() && cell < total {
        let ca = body[pos];
        if (b'k'..=b'z').contains(&ca) {
            // Run of white cells; `k` skips one cell and `z` skips sixteen.
            cell += (ca - b'k') as usize + 1;
            pos += 1;
            continue;
        }

        let (down, right, consumed) = if ca == b'.' {
            (None, None, 1)
        } else {
            let down = decode_kakuro_num(ca);
            let right = body.get(pos + 1).copied().and_then(decode_kakuro_num);
            (down, right, 2)
        };
        grid[cell / cols][cell % cols] = Some((down, right));
        cell += 1;
        pos += consumed;
    }

    let mut rem_seq = vec![];
    for x in 0..cols {
        if grid[0][x].is_none() {
            rem_seq.push(decode_kakuro_num(*body.get(pos)?));
            pos += 1;
        }
    }
    for y in 0..rows {
        if grid[y][0].is_none() {
            rem_seq.push(decode_kakuro_num(*body.get(pos)?));
            pos += 1;
        }
    }

    Some((grid, rem_seq, &body[pos..]))
}

/// Decode the consecutive-bar segment of a Consecutive Kakuro URL.  Each
/// playable cell contributes one pzpr number16 value whose bit 1 (value 2)
/// marks the bar on its right border and bit 0 (value 1) the bar on its
/// bottom border.  The editor only emits the values 0..3 as a single
/// hexadecimal digit per cell, but run-length skip letters are accepted as
/// well for compatibility with the generic pzpr decoder.
fn decode_consecutive_bars(body: &[u8], rows: usize, cols: usize) -> Vec<Vec<u8>> {
    let total = rows * cols;
    let mut bars = vec![vec![0u8; cols]; rows];
    let mut index = 0usize;
    let mut pos = 0usize;
    while index < total && pos < body.len() {
        let ca = body[pos];
        if let Some(value) = hex_digit(ca) {
            bars[index / cols][index % cols] = value as u8;
            index += 1;
        } else if (b'g'..=b'z').contains(&ca) {
            index += (ca - b'g') as usize + 1;
        }
        pos += 1;
    }
    bars
}

pub fn deserialize_consecutive_problem(url: &str) -> Option<(Problem, Vec<Vec<u8>>)> {
    let serialized = strip_prefix(url)?;
    let mut parts = serialized.split('/');
    if parts.next()? != "consecutivekakuro" {
        return None;
    }
    let cols: usize = parts.next()?.parse().ok()?;
    let rows: usize = parts.next()?.parse().ok()?;
    let body = parts.next().unwrap_or("").as_bytes();

    if cols == 0 || rows == 0 {
        return None;
    }

    let (intermediate_grid, rem_seq, remaining) = decode_kakuro_body(body, rows, cols)?;
    let problem = problem_from_intermediate(intermediate_grid, rem_seq)?;
    let bars = decode_consecutive_bars(remaining, rows, cols);

    Some((problem, bars))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Vec<Vec<Option<KakuroClue>>> {
        // https://puzz.link/p?kakuro/6/5/Dclh4t9fl3-p-gl-alJeC3BgG
        let mut ret = vec![vec![None; 7]; 6];
        ret[0][0] = Some(KakuroClue {
            down: None,
            right: None,
        });
        ret[0][1] = Some(KakuroClue {
            down: None,
            right: None,
        });
        ret[0][2] = Some(KakuroClue {
            down: Some(29),
            right: None,
        });
        ret[0][3] = Some(KakuroClue {
            down: Some(14),
            right: None,
        });
        ret[0][4] = Some(KakuroClue {
            down: None,
            right: None,
        });
        ret[0][5] = Some(KakuroClue {
            down: Some(22),
            right: None,
        });
        ret[0][6] = Some(KakuroClue {
            down: Some(3),
            right: None,
        });
        ret[1][0] = Some(KakuroClue {
            down: None,
            right: None,
        });
        ret[1][1] = Some(KakuroClue {
            down: Some(23),
            right: Some(12),
        });
        ret[1][4] = Some(KakuroClue {
            down: Some(17),
            right: Some(4),
        });
        ret[2][0] = Some(KakuroClue {
            down: None,
            right: Some(21),
        });
        ret[3][0] = Some(KakuroClue {
            down: None,
            right: Some(16),
        });
        ret[3][3] = Some(KakuroClue {
            down: Some(9),
            right: Some(15),
        });
        ret[3][6] = Some(KakuroClue {
            down: Some(3),
            right: None,
        });
        ret[4][0] = Some(KakuroClue {
            down: None,
            right: Some(26),
        });
        ret[5][0] = Some(KakuroClue {
            down: None,
            right: None,
        });
        ret[5][1] = Some(KakuroClue {
            down: None,
            right: Some(16),
        });
        ret[5][4] = Some(KakuroClue {
            down: None,
            right: Some(10),
        });

        ret
    }

    #[test]
    fn test_kakuro_problem() {
        let problem = problem_for_tests();
        let ans = solve_kakuro(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        let expected = crate::util::tests::to_option_2d([
            [0, 0, 0, 0, 0, 0, 0],
            [0, 0, 3, 9, 0, 3, 1],
            [0, 6, 4, 5, 3, 1, 2],
            [0, 9, 7, 0, 9, 6, 0],
            [0, 8, 6, 2, 5, 4, 1],
            [0, 0, 9, 7, 0, 8, 2],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_kakuro_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?kakuro/6/5/Dclh4t9fl3-p-gl-alJeC3BgG";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }

    #[test]
    fn test_consecutivekakuro_alias_deserializes() {
        let url = "https://puzz.link/p?consecutivekakuro/6/5/Dclh4t9fl3-p-gl-alJeC3BgG";
        assert!(deserialize_problem(url).is_some());
    }

    #[test]
    fn test_kakuro_native_zero_length_clues() {
        // Native PuzzLink Kakuro URLs can contain explicit zero clues.  A
        // zero-length run (no cells before the next clue) is valid and must
        // simply be skipped by the solver.
        let url = "https://puzz.link/p?kakuro/5/5/48la0.na0lh3l0Bn.0cl.c4a3";
        let problem = deserialize_problem(url).expect("valid native Kakuro URL");
        assert!(problem.iter().flatten().any(|clue| {
            clue.map(|clue| clue.down == Some(0) || clue.right == Some(0))
                .unwrap_or(false)
        }));
        assert!(solve_kakuro(&problem).is_some());
    }

    #[test]
    fn test_consecutivekakuro_parses_bars() {
        // Clue cell at the top-left followed by a fully white 3x3 board,
        // with bars on the right and bottom borders of the clue cell.
        let url = "https://puzz.link/p?consecutivekakuro/3/3/34r----300000000";
        let (problem, bars) = deserialize_consecutive_problem(url).unwrap();

        assert_eq!(problem.len(), 4);
        assert!(problem.iter().all(|row| row.len() == 4));
        assert_eq!(
            problem[1][1].map(|c| (c.down, c.right)),
            Some((Some(3), Some(4)))
        );

        assert_eq!(bars.len(), 3);
        assert!(bars.iter().all(|row| row.len() == 3));
        assert_eq!(bars[0][0], 3);
        assert_eq!(bars[0][1], 0);
        assert_eq!(bars[1][0], 0);
        assert_eq!(bars[1][1], 0);
    }

    #[test]
    fn test_consecutivekakuro_solver_enforces_bars() {
        // 2x2 all-white board:
        //   a b  (row sum 4)     a b = 1 3
        //   c d  (row sum 6)     c d = 2 4
        // col sums 3 and 7.  The unique plain solution is shown on the right;
        // bars mark the consecutive pairs a-c and b-d.
        let clues = vec![
            vec![
                Some(KakuroClue {
                    down: None,
                    right: None,
                }),
                Some(KakuroClue {
                    down: Some(3),
                    right: None,
                }),
                Some(KakuroClue {
                    down: Some(7),
                    right: None,
                }),
            ],
            vec![
                Some(KakuroClue {
                    down: None,
                    right: Some(4),
                }),
                None,
                None,
            ],
            vec![
                Some(KakuroClue {
                    down: None,
                    right: Some(6),
                }),
                None,
                None,
            ],
        ];
        let expected = crate::util::tests::to_option_2d([[0, 0, 0], [0, 1, 3], [0, 2, 4]]);

        assert_eq!(solve_kakuro(&clues), Some(expected.clone()));

        // The bars match the unique plain solution, so solving with bars
        // keeps the same answer.
        let bars = vec![vec![1, 1], vec![0, 0]];
        assert_eq!(
            solve_kakuro_with_bars(&clues, Some(&bars)),
            Some(expected.clone())
        );

        // Requiring a bar between a and b (1 and 3 are not consecutive)
        // makes the puzzle unsatisfiable.
        let contradictory = vec![vec![3, 1], vec![0, 0]];
        assert_eq!(solve_kakuro_with_bars(&clues, Some(&contradictory)), None);

        // Removing the bar between a and c (1 and 2 are consecutive, so the
        // pair must not be consecutive without a bar) is also unsolvable.
        let contradictory = vec![vec![0, 1], vec![0, 0]];
        assert_eq!(solve_kakuro_with_bars(&clues, Some(&contradictory)), None);
    }

    #[test]
    fn test_consecutivekakuro_decodes_interior_clues_and_bars() {
        // 4x4 board with clue cells at (0,0) and (1,1), produced by the pzpr
        // editor.  The body exercises run-length letters (`n`, `t`) between
        // clue cells plus the trailing 16-cell bar segment.
        let url = "https://puzz.link/p?consecutivekakuro/4/4/67n98t------0220000002000000";
        let (problem, bars) = deserialize_consecutive_problem(url).unwrap();

        assert_eq!(problem.len(), 5);
        assert!(problem.iter().all(|row| row.len() == 5));
        assert_eq!(
            problem[1][1].map(|c| (c.down, c.right)),
            Some((Some(6), Some(7)))
        );
        assert_eq!(
            problem[2][2].map(|c| (c.down, c.right)),
            Some((Some(9), Some(8)))
        );
        assert_eq!(problem[1][2], None);

        assert_eq!(bars.len(), 4);
        assert_eq!(bars[0], vec![0, 2, 2, 0]);
        assert_eq!(bars[2], vec![0, 2, 0, 0]);
    }

    #[test]
    fn test_consecutivekakuro_end_to_end_url() {
        // A URL produced by the pzpr editor for the 2x2 puzzle above:
        // clue grid `n` (four white cells), outside clues 3/7 (down) and
        // 4/6 (right), bars `1100` (a-c and b-d).
        let url = "https://puzz.link/p?consecutivekakuro/2/2/n37461100";
        let (problem, bars) = deserialize_consecutive_problem(url).unwrap();
        assert_eq!(bars, vec![vec![1, 1], vec![0, 0]]);

        let expected = crate::util::tests::to_option_2d([[0, 0, 0], [0, 1, 3], [0, 2, 4]]);
        assert_eq!(
            solve_kakuro_with_bars(&problem, Some(&bars)),
            Some(expected)
        );
    }
}
