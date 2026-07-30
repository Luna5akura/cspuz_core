use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{
    problem_to_url, url_to_problem, Choice, Combinator, Dict, Grid, HexInt, Optionalize, Spaces,
};
use cspuz_rs::solver::{BoolVarArray2D, Solver};

pub fn solve_lakes(clues: &[Vec<Option<i32>>]) -> Option<Vec<Vec<Option<bool>>>> {
    let (h, w) = util::infer_shape(clues);

    let mut solver = Solver::new();
    let is_black = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(is_black);

    add_constraints(clues, &mut solver, is_black);

    solver.irrefutable_facts().map(|f| f.get(is_black))
}

fn add_constraints(clues: &[Vec<Option<i32>>], solver: &mut Solver, is_black: &BoolVarArray2D) {
    let (h, w) = util::infer_shape(clues);

    let mut clue_pos = vec![];
    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                clue_pos.push((y, x, n));
            }
        }
    }

    let group_id = solver.int_var_2d((h, w), 0, clue_pos.len() as i32);
    solver.add_expr(is_black.iff(group_id.eq(0)));

    for i in 1..=clue_pos.len() {
        graph::active_vertices_connected_2d(solver, group_id.eq(i as i32));
    }

    if h >= 2 {
        solver.add_expr(
            (!is_black.conv2d_or((2, 1))).imp(
                group_id
                    .slice((..(h - 1), ..))
                    .eq(group_id.slice((1.., ..))),
            ),
        );
    }
    if w >= 2 {
        solver.add_expr(
            (!is_black.conv2d_or((1, 2))).imp(
                group_id
                    .slice((.., ..(w - 1)))
                    .eq(group_id.slice((.., 1..))),
            ),
        );
    }

    for (i, &(y, x, n)) in clue_pos.iter().enumerate() {
        solver.add_expr(group_id.at((y, x)).eq((i + 1) as i32));
        if n > 0 {
            solver.add_expr(group_id.eq((i + 1) as i32).count_true().eq(n));
        }
    }
}

type Problem = Vec<Vec<Option<i32>>>;

fn combinator() -> impl Combinator<Problem> {
    Grid::new(Choice::new(vec![
        Box::new(Optionalize::new(HexInt)),
        Box::new(Spaces::new(None, 'g')),
        Box::new(Dict::new(Some(-1), ".")),
    ]))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    problem_to_url(combinator(), "lakes", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["lakes"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    fn problem_for_tests() -> Problem {
        vec![
            vec![Some(2), None, Some(2)],
            vec![None, None, None],
            vec![None, None, None],
        ]
    }

    #[test]
    #[rustfmt::skip]
    fn test_lakes_problem() {
        let problem = problem_for_tests();
        let ans = solve_lakes(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();

        let expected = crate::util::tests::to_option_bool_2d([
            [0, 1, 0],
            [0, 1, 0],
            [1, 1, 1],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_lakes_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?lakes/3/3/2g2l";
        crate::util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
