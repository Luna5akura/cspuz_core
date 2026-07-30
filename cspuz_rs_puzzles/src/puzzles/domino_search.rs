use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{
    problem_to_url, url_to_problem, Choice, Combinator, Dict, Grid, HexInt, Optionalize, Spaces,
};
use cspuz_rs::solver::{count_true, Solver};

type Problem = Vec<Vec<Option<i32>>>;

fn pair_key(a: i32, b: i32) -> (i32, i32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

pub fn solve_domino_search(
    clues: &[Vec<Option<i32>>],
) -> Option<graph::BoolInnerGridEdgesIrrefutableFacts> {
    let (h, w) = util::infer_shape(clues);

    let mut min_clue: Option<i32> = None;
    let mut max_clue: Option<i32> = None;
    let mut active_count = 0;
    for row in clues {
        for clue in row {
            let Some(clue) = *clue else {
                continue;
            };
            if clue < 0 {
                return None;
            }
            active_count += 1;
            min_clue = Some(min_clue.map_or(clue, |min| min.min(clue)));
            max_clue = Some(max_clue.map_or(clue, |max| max.max(clue)));
        }
    }
    let min_clue = min_clue?;
    let max_clue = max_clue?;

    let num_values = (max_clue - min_clue + 1) as usize;
    if active_count != num_values * (num_values + 1) {
        return None;
    }

    let mut solver = Solver::new();
    let is_border = graph::BoolInnerGridEdges::new(&mut solver, (h, w));
    solver.add_answer_key_bool(&is_border.horizontal);
    solver.add_answer_key_bool(&is_border.vertical);

    for y in 0..h {
        for x in 0..w {
            if clues[y][x].is_none() {
                continue;
            }

            let mut neighbors = vec![];
            if y > 0 && clues[y - 1][x].is_some() {
                neighbors.push(!is_border.horizontal.at((y - 1, x)));
            }
            if y < h - 1 && clues[y + 1][x].is_some() {
                neighbors.push(!is_border.horizontal.at((y, x)));
            }
            if x > 0 && clues[y][x - 1].is_some() {
                neighbors.push(!is_border.vertical.at((y, x - 1)));
            }
            if x < w - 1 && clues[y][x + 1].is_some() {
                neighbors.push(!is_border.vertical.at((y, x)));
            }
            solver.add_expr(count_true(neighbors).eq(1));
        }
    }

    for y in 0..h {
        for x in 0..w {
            if y < h - 1 && (clues[y][x].is_none() || clues[y + 1][x].is_none()) {
                solver.add_expr(is_border.horizontal.at((y, x)));
            }
            if x < w - 1 && (clues[y][x].is_none() || clues[y][x + 1].is_none()) {
                solver.add_expr(is_border.vertical.at((y, x)));
            }
        }
    }

    for a in min_clue..=max_clue {
        for b in a..=max_clue {
            let mut candidates = vec![];
            for y in 0..h {
                for x in 0..w {
                    if y < h - 1
                        && clues[y][x].is_some()
                        && clues[y + 1][x].is_some()
                        && pair_key(clues[y][x].unwrap(), clues[y + 1][x].unwrap()) == (a, b)
                    {
                        candidates.push(!is_border.horizontal.at((y, x)));
                    }
                    if x < w - 1
                        && clues[y][x].is_some()
                        && clues[y][x + 1].is_some()
                        && pair_key(clues[y][x].unwrap(), clues[y][x + 1].unwrap()) == (a, b)
                    {
                        candidates.push(!is_border.vertical.at((y, x)));
                    }
                }
            }
            solver.add_expr(count_true(candidates).eq(1));
        }
    }

    solver.irrefutable_facts().map(|f| f.get(&is_border))
}

fn combinator() -> impl Combinator<Problem> {
    Grid::new(Choice::new(vec![
        Box::new(Optionalize::new(HexInt)),
        Box::new(Spaces::new(None, 'g')),
        Box::new(Dict::new(Some(-1), ".")),
    ]))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    problem_to_url(combinator(), "domino-search", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["domino-search"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    fn problem_for_tests() -> Problem {
        vec![
            vec![Some(0), Some(0), Some(0), Some(1)],
            vec![Some(1), Some(1), Some(0), Some(2)],
            vec![Some(1), Some(2), Some(2), Some(2)],
        ]
    }

    #[test]
    fn test_domino_search_problem() {
        let problem = problem_for_tests();
        assert!(solve_domino_search(&problem).is_some());
    }

    #[test]
    fn test_domino_search_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?domino-search/4/3/000111021222";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
