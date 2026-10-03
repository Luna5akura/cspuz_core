use crate::util;
use cspuz_rs::serializer::{
    problem_to_url, url_to_problem, Choice, Combinator, Grid, HexInt, Optionalize, Spaces,
};
use cspuz_rs::solver::Solver;

pub fn solve_star_gazing(clues: &[Vec<Option<i32>>]) -> Option<Vec<Vec<Option<bool>>>> {
    let (h, w) = util::infer_shape(clues);

    let mut solver = Solver::new();
    let is_star = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(is_star);

    // No two stars may touch each other, even diagonally.
    for y in 0..h {
        for x in 0..w {
            for &(dy, dx) in &[(0, 1), (1, 1), (1, 0), (1, -1)] {
                let y2 = y as i32 + dy;
                let x2 = x as i32 + dx;
                if 0 <= y2 && y2 < h as i32 && 0 <= x2 && x2 < w as i32 {
                    solver.add_expr(!(is_star.at((y, x)) & is_star.at((y2 as usize, x2 as usize))));
                }
            }
        }
    }

    // A number indicates the total number of stars visible from the cell in
    // the four directions up to the edges of the board, i.e. the number of
    // stars in the same row plus the same column. The clue cell itself
    // cannot contain a star (so it is not double counted).
    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                solver.add_expr(!is_star.at((y, x)));
                let row_count = is_star.slice_fixed_y((y, ..)).count_true();
                let col_count = is_star.slice_fixed_x((.., x)).count_true();
                solver.add_expr((row_count + col_count).eq(n));
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(is_star))
}

type Problem = Vec<Vec<Option<i32>>>;

fn combinator() -> impl Combinator<Problem> {
    Grid::new(Choice::new(vec![
        Box::new(Optionalize::new(HexInt)),
        Box::new(Spaces::new(None, 'g')),
    ]))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    problem_to_url(combinator(), "stargazing", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["stargazing"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        vec![
            vec![Some(1), None, None, Some(2), None],
            vec![None, None, None, None, None],
            vec![None, None, Some(1), None, Some(2)],
            vec![None, None, None, None, None],
            vec![None, Some(2), None, None, Some(2)],
        ]
    }

    #[test]
    fn test_star_gazing_problem() {
        let problem = problem_for_tests();
        let ans = solve_star_gazing(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        let expected = crate::util::tests::to_option_bool_2d([
            [0, 0, 0, 0, 1],
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 1, 0],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_star_gazing_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?stargazing/5/5/1h2n1g2l2h2";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
