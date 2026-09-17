//! Solver and PuzzLink serializer for the fixed-size Neighbors puzzle.
//!
//! A Neighbors board is always 9x9.  Every cell contains a number from 1 to
//! 3, every row and column contains each number exactly three times, and the
//! adjacency rule depends on whether the cell is outlined (gray): white cells
//! need at least one orthogonal equal neighbour, while outlined cells may not
//! have an orthogonal equal neighbour.

use cspuz_rs::serializer::{
    problem_to_url_with_context, url_to_problem, Choice, Combinator, Context, ContextBasedGrid,
    Dict, PrefixAndSuffix, Size, Tuple2,
};
use cspuz_rs::solver::Solver;

pub const NEIGHBOR_SIZE: usize = 9;

/// `(givens, outlined)` for a Neighbors puzzle.
///
/// `None` in `givens` denotes a cell without a fixed clue.  `true` in the
/// outlined grid denotes a gray/outlined cell.
pub type Problem = (Vec<Vec<Option<i32>>>, Vec<Vec<bool>>);

fn is_9x9<T>(grid: &[Vec<T>]) -> bool {
    grid.len() == NEIGHBOR_SIZE && grid.iter().all(|row| row.len() == NEIGHBOR_SIZE)
}

fn valid_problem(problem: &Problem) -> bool {
    is_9x9(&problem.0)
        && is_9x9(&problem.1)
        && problem
            .0
            .iter()
            .flatten()
            .all(|&n| n.is_none_or(|n| (1..=3).contains(&n)))
}

/// Solve a Neighbors board and return the (possibly partially known) answer
/// grid.  The current constraints make every answer cell an integer, so a
/// successful result contains `Some(1..=3)` in every position.
pub fn solve_neighbor(
    givens: &[Vec<Option<i32>>],
    outlined: &[Vec<bool>],
) -> Option<Vec<Vec<Option<i32>>>> {
    if !is_9x9(givens) || !is_9x9(outlined) {
        return None;
    }
    if givens
        .iter()
        .flatten()
        .any(|&n| n.is_some_and(|n| !(1..=3).contains(&n)))
    {
        return None;
    }

    let mut solver = Solver::new();
    let num = &solver.int_var_2d((NEIGHBOR_SIZE, NEIGHBOR_SIZE), 1, 3);
    solver.add_answer_key_int(num);

    for y in 0..NEIGHBOR_SIZE {
        for x in 0..NEIGHBOR_SIZE {
            if let Some(n) = givens[y][x] {
                solver.add_expr(num.at((y, x)).eq(n));
            }
        }
    }

    // Every row and column contains exactly three copies of each digit.
    for n in 1..=3 {
        for y in 0..NEIGHBOR_SIZE {
            solver.add_expr(num.slice_fixed_y((y, ..)).eq(n).count_true().eq(3));
        }
        for x in 0..NEIGHBOR_SIZE {
            solver.add_expr(num.slice_fixed_x((.., x)).eq(n).count_true().eq(3));
        }
    }

    // Compare each cell with its existing orthogonal neighbours.  The
    // four_neighbors helper naturally omits neighbours outside the board.
    for y in 0..NEIGHBOR_SIZE {
        for x in 0..NEIGHBOR_SIZE {
            let same = num.four_neighbors((y, x)).eq(num.at((y, x))).count_true();
            if outlined[y][x] {
                solver.add_expr(same.eq(0));
            } else {
                solver.add_expr(same.ge(1));
            }
        }
    }

    solver.irrefutable_facts().map(|facts| facts.get(num))
}

/// Convenience wrapper accepting the tuple representation used by the URL
/// serializer.
pub fn solve_neighbors(problem: &Problem) -> Option<Vec<Vec<Option<i32>>>> {
    solve_neighbor(&problem.0, &problem.1)
}

/// Alias used by callers that treat each puzzle module uniformly.
pub fn solve(problem: &Problem) -> Option<Vec<Vec<Option<i32>>>> {
    solve_neighbors(problem)
}

fn combinator() -> impl Combinator<Problem> {
    // Keep both grids explicit (one character per cell) so this format is
    // readable by pzprjs as well as by the Rust backend:
    //   <givens>/<outlined-mask>
    Size::new(Tuple2::new(
        ContextBasedGrid::new(Choice::new(vec![
            Box::new(Dict::new(None, ".")),
            Box::new(Dict::new(Some(1), "1")),
            Box::new(Dict::new(Some(2), "2")),
            Box::new(Dict::new(Some(3), "3")),
        ])),
        PrefixAndSuffix::new(
            "/",
            ContextBasedGrid::new(Choice::new(vec![
                Box::new(Dict::new(false, "0")),
                Box::new(Dict::new(true, "1")),
            ])),
            "",
        ),
    ))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    if !valid_problem(problem) {
        return None;
    }
    problem_to_url_with_context(
        combinator(),
        "neighbors",
        problem.clone(),
        &Context::sized(NEIGHBOR_SIZE, NEIGHBOR_SIZE),
    )
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let problem = url_to_problem(combinator(), &["neighbors", "neighbor"], url)?;
    valid_problem(&problem).then_some(problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL_19: &str = "https://puzz.link/p?neighbor/9/9/............3.......2.......1...3.......2.......1...3.......2.......1............/111111101001110001011110001110111111110000101101111001100001111110101001000110000";
    const URL_20: &str = "https://puzz.link/p?neighbor/9/9/..........1.1...........2...1...........2...........3...2...........3.3........../110100100110111101001001100110000101110001100111101110110011001000111010100011100";

    fn empty_problem() -> Problem {
        (vec![vec![None; 9]; 9], vec![vec![false; 9]; 9])
    }

    #[test]
    fn serializer_roundtrip() {
        let problem = empty_problem();
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(
            url,
            format!(
                "https://puzz.link/p?neighbor/9/9/{}/{}",
                ".".repeat(81),
                "0".repeat(81)
            )
        );
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn rejects_non_9x9() {
        assert!(deserialize_problem("https://puzz.link/p?neighbor/8/9/................................................................/000000000000000000000000000000000000000000000000000000000000000000000000000000000").is_none());

        let malformed = (
            vec![vec![None; NEIGHBOR_SIZE]; NEIGHBOR_SIZE - 1],
            vec![vec![false; NEIGHBOR_SIZE]; NEIGHBOR_SIZE],
        );
        assert!(serialize_problem(&malformed).is_none());
        assert!(solve_neighbors(&malformed).is_none());

        let mut invalid_clue = empty_problem();
        invalid_clue.0[0][0] = Some(4);
        assert!(serialize_problem(&invalid_clue).is_none());
        assert!(solve_neighbors(&invalid_clue).is_none());
    }

    #[test]
    fn solves_built_in_examples() {
        let p19 = deserialize_problem(URL_19).unwrap();
        assert!(solve_neighbors(&p19).is_some());
        // URL aliases are accepted, but serialization remains canonical.
        let alias = URL_19.replacen("neighbor/", "neighbors/", 1);
        assert_eq!(deserialize_problem(&alias), Some(p19));

        // Keep a second fixture to catch accidental assumptions about the
        // number/outline distribution.
        let p20 = deserialize_problem(URL_20).unwrap();
        assert!(solve_neighbors(&p20).is_some());
    }
}
