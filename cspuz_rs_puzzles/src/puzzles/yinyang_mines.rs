use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{problem_to_url, url_to_problem, Combinator, Grid, Map, MultiDigit};
use cspuz_rs::solver::{count_true, Solver};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YinYangMinesClue {
    None,
    White,
    Black,
    /// A white circle with a number: the number of black cells among the
    /// eight neighboring cells.
    Number(i32),
}

pub fn solve_yinyang_mines(clues: &[Vec<YinYangMinesClue>]) -> Option<Vec<Vec<Option<bool>>>> {
    let (h, w) = util::infer_shape(clues);

    let mut solver = Solver::new();
    let is_black = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(is_black);

    // Standard Yin-Yang rules: both colors are connected, no 2x2 block of a
    // single color, and no checkerboard pattern in any 2x2 block.
    graph::active_vertices_connected_2d(&mut solver, is_black);
    solver.add_expr(!is_black.conv2d_and((2, 2)));

    graph::active_vertices_connected_2d(&mut solver, !is_black);
    solver.add_expr(!(!is_black).conv2d_and((2, 2)));

    solver.add_expr(
        !(is_black.slice((..(h - 1), ..(h - 1)))
            & is_black.slice((1.., 1..))
            & !is_black.slice((..(h - 1), 1..))
            & !is_black.slice((1.., ..(h - 1)))),
    );
    solver.add_expr(
        !(!is_black.slice((..(h - 1), ..(h - 1)))
            & !is_black.slice((1.., 1..))
            & is_black.slice((..(h - 1), 1..))
            & is_black.slice((1.., ..(h - 1)))),
    );

    for y in 0..h {
        for x in 0..w {
            let p = (y, x);
            match clues[y][x] {
                YinYangMinesClue::None => (),
                YinYangMinesClue::White => {
                    solver.add_expr(!is_black.at(p));
                }
                YinYangMinesClue::Black => {
                    solver.add_expr(is_black.at(p));
                }
                YinYangMinesClue::Number(n) => {
                    solver.add_expr(!is_black.at(p));
                    let mut neighbors = vec![];
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dy == 0 && dx == 0 {
                                continue;
                            }
                            let y2 = y as i32 + dy;
                            let x2 = x as i32 + dx;
                            if 0 <= y2 && y2 < h as i32 && 0 <= x2 && x2 < w as i32 {
                                neighbors.push(is_black.at((y2 as usize, x2 as usize)).expr());
                            }
                        }
                    }
                    solver.add_expr(count_true(neighbors).eq(n));
                }
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(is_black))
}

type Problem = Vec<Vec<YinYangMinesClue>>;

fn combinator() -> impl Combinator<Vec<Vec<YinYangMinesClue>>> {
    Grid::new(Map::new(
        MultiDigit::new(12, 1),
        |x: YinYangMinesClue| {
            Some(match x {
                YinYangMinesClue::None => 0,
                YinYangMinesClue::White => 1,
                YinYangMinesClue::Black => 2,
                YinYangMinesClue::Number(n) => 3 + n,
            })
        },
        |n: i32| match n {
            0 => Some(YinYangMinesClue::None),
            1 => Some(YinYangMinesClue::White),
            2 => Some(YinYangMinesClue::Black),
            3..=11 => Some(YinYangMinesClue::Number(n - 3)),
            _ => None,
        },
    ))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    problem_to_url(combinator(), "yinyangmines", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["yinyangmines"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Vec<Vec<YinYangMinesClue>> {
        let mut ret = vec![vec![YinYangMinesClue::None; 5]; 5];
        ret[0][0] = YinYangMinesClue::Black;
        ret[1][1] = YinYangMinesClue::Number(6);
        ret[1][3] = YinYangMinesClue::Number(6);
        ret[2][1] = YinYangMinesClue::Number(5);
        ret[2][2] = YinYangMinesClue::Number(2);
        ret[2][3] = YinYangMinesClue::Number(5);
        ret[3][1] = YinYangMinesClue::Number(6);
        ret[3][3] = YinYangMinesClue::Number(6);
        ret
    }

    #[test]
    fn test_yinyang_mines_problem() {
        let problem = problem_for_tests();
        let ans = solve_yinyang_mines(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        let expected = crate::util::tests::to_option_bool_2d([
            [1, 1, 1, 1, 1],
            [1, 0, 1, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 1, 0, 1],
            [1, 1, 1, 1, 1],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_yinyang_mines_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?yinyangmines/5/5/2000009090085800909000000";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
