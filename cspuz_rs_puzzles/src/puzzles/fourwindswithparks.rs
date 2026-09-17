use crate::util;
use cspuz_rs::serializer::{
    problem_to_url, url_to_problem, Choice, Combinator, Grid, HexInt, Optionalize, Spaces,
};
use cspuz_rs::solver::{consecutive_prefix_true, sum, Solver};

/// Direction values used in the answer grid: 0 is the one park cell in a
/// row/column, and 1..=4 are up, down, left and right respectively.
pub fn solve_fourwindswithparks(clues: &[Vec<Option<i32>>]) -> Option<Vec<Vec<Option<i32>>>> {
    if clues.is_empty() || clues[0].is_empty() {
        return None;
    }
    let (h, w) = util::infer_shape(clues);
    if h == 0 || w == 0 || clues.iter().any(|row| row.len() != w) {
        return None;
    }
    // Four Winds with Parks clues are non-negative totals.  A negative value
    // is not a wildcard in this puzzle (an absent clue is represented by
    // `None`).
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

    // Numbered cells are occupied by clues, not arrows.
    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_some() {
                solver.add_expr(dir.at((y, x)).eq(0));
            }
        }
    }

    // Every row and column contains exactly one uncovered (park) cell.
    for y in 0..h {
        let parks = (0..w)
            .filter(|&x| clues[y][x].is_none())
            .map(|x| dir.at((y, x)).eq(0))
            .collect::<Vec<_>>();
        solver.add_expr(cspuz_rs::solver::count_true(parks).eq(1));
    }
    for x in 0..w {
        let parks = (0..h)
            .filter(|&y| clues[y][x].is_none())
            .map(|y| dir.at((y, x)).eq(0))
            .collect::<Vec<_>>();
        solver.add_expr(cspuz_rs::solver::count_true(parks).eq(1));
    }

    // An arrow cell must be connected backwards to a numbered cell.  This
    // prevents arrows from starting in the middle of an otherwise empty run.
    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_some() {
                continue;
            }
            let d = dir.at((y, x));
            if y + 1 == h {
                solver.add_expr(d.ne(1));
            } else if clues[y + 1][x].is_none() {
                solver.add_expr(d.eq(1).imp(dir.at((y + 1, x)).eq(1)));
            }
            if y == 0 {
                solver.add_expr(d.ne(2));
            } else if clues[y - 1][x].is_none() {
                solver.add_expr(d.eq(2).imp(dir.at((y - 1, x)).eq(2)));
            }
            if x + 1 == w {
                solver.add_expr(d.ne(3));
            } else if clues[y][x + 1].is_none() {
                solver.add_expr(d.eq(3).imp(dir.at((y, x + 1)).eq(3)));
            }
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

    solver.irrefutable_facts().map(|f| f.get(dir))
}

type Problem = Vec<Vec<Option<i32>>>;

fn combinator() -> impl Combinator<Problem> {
    Grid::new(Choice::new(vec![
        Box::new(Optionalize::new(HexInt)),
        Box::new(Spaces::new(None, 'g')),
    ]))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    if problem.is_empty() || problem[0].is_empty() {
        return None;
    }
    let (h, w) = util::infer_shape(problem);
    if h == 0 || w == 0 || problem.iter().any(|row| row.len() != w) {
        return None;
    }
    if problem
        .iter()
        .flatten()
        .any(|clue| clue.is_some_and(|n| n < 0))
    {
        return None;
    }
    problem_to_url(combinator(), "fourwindswithparks", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["fourwindswithparks"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fourwindswithparks_problem() {
        // Three clues admit the following arrows and one park in every row
        // and column.
        let clues = vec![
            vec![Some(0), None, None],
            vec![None, None, Some(2)],
            vec![None, Some(1), None],
        ];
        let ans = solve_fourwindswithparks(&clues).unwrap();
        for y in 0..3 {
            assert_eq!(
                (0..3)
                    .filter(|&x| clues[y][x].is_none() && ans[y][x] == Some(0))
                    .count(),
                1
            );
        }
        for x in 0..3 {
            assert_eq!(
                (0..3)
                    .filter(|&y| clues[y][x].is_none() && ans[y][x] == Some(0))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn test_fourwindswithparks_serializer() {
        let problem = vec![vec![Some(1), None], vec![None, Some(0)]];
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn rejects_negative_clues() {
        let problem = vec![vec![Some(-1), None], vec![None, Some(0)]];
        assert!(solve_fourwindswithparks(&problem).is_none());
        assert!(serialize_problem(&problem).is_none());
    }

    #[test]
    fn rejects_empty_or_ragged_grids() {
        let empty: Problem = vec![];
        assert!(solve_fourwindswithparks(&empty).is_none());
        assert!(serialize_problem(&empty).is_none());
        let ragged = vec![vec![None, None], vec![None]];
        assert!(solve_fourwindswithparks(&ragged).is_none());
        assert!(serialize_problem(&ragged).is_none());
    }
}
