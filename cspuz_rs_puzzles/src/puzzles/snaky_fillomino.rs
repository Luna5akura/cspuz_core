use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{
    problem_to_url, url_to_problem, Choice, Combinator, Dict, Grid, HexInt, Optionalize, Spaces,
};
use cspuz_rs::solver::{any, Config, GraphDivisionMode, IntExpr, Solver};

pub fn solve_snaky_fillomino(
    clues: &[Vec<Option<i32>>],
) -> Option<(
    Vec<Vec<Option<i32>>>,
    graph::BoolInnerGridEdgesIrrefutableFacts,
)> {
    let (h, w) = util::infer_shape(clues);

    // Use the default (Cpp) graph division mode: the Rust-mode custom
    // propagator is incomplete and can admit models with inconsistent
    // region sizes on weakly-clued boards.
    let config = Config::default();

    let mut solver = Solver::with_config(config);
    let mut ranges = vec![];
    for y in 0..h {
        let mut row = vec![];
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                if n < 0 {
                    row.push((1, (h * w) as i32));
                } else {
                    row.push((n, n));
                }
            } else {
                row.push((1, (h * w) as i32));
            }
        }
        ranges.push(row);
    }
    let num = &solver.int_var_2d_from_ranges((h, w), &ranges);
    solver.add_answer_key_int(num);

    let is_border = graph::BoolInnerGridEdges::new(&mut solver, (h, w));
    solver.add_answer_key_bool(&is_border.horizontal);
    solver.add_answer_key_bool(&is_border.vertical);
    solver.add_expr(
        num.slice((.., ..(w - 1)))
            .ne(num.slice((.., 1..)))
            .iff(&is_border.vertical),
    );
    solver.add_expr(
        num.slice((..(h - 1), ..))
            .ne(num.slice((1.., ..)))
            .iff(&is_border.horizontal),
    );

    graph::graph_division_2d(&mut solver, num, &is_border);

    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                if n >= 0 {
                    solver.add_expr(num.at((y, x)).eq(n));
                }
            }
        }
    }

    // ----- Snaky Fillomino constraints -----
    //
    // Every region must be a "snake": a path with exactly two endpoints
    // (cells with exactly one same-region neighbor each). This means:
    //   1. No branching: each cell has at most 2 same-region neighbors.
    //   2. No 2x2 block inside a region.
    //   3. No closed loop (a loop has no endpoint at all).

    // For each cell, the number of orthogonally adjacent cells that belong
    // to the same region (i.e., carry the same number).
    let mut deg: Vec<Vec<IntExpr>> = vec![];
    for y in 0..h {
        let mut row = vec![];
        for x in 0..w {
            let eq_neighbors = num.four_neighbors((y, x)).eq(num.at((y, x)));
            let d = eq_neighbors.count_true();
            solver.add_expr(d.le(2));
            row.push(d);
        }
        deg.push(row);
    }

    // No 2x2 block of cells in the same region: of the four cells of any
    // 2x2 block, the three equalities along a spanning path must not all
    // hold simultaneously.
    for y in 0..(h - 1) {
        for x in 0..(w - 1) {
            let e1 = num.at((y, x)).eq(num.at((y, x + 1)));
            let e2 = num.at((y, x + 1)).eq(num.at((y + 1, x + 1)));
            let e3 = num.at((y + 1, x)).eq(num.at((y + 1, x + 1)));
            solver.add_expr(!(e1 & e2 & e3));
        }
    }

    // No closed loops: assign each cell a "rank"; every non-root cell
    // (rank > 0) must have a same-region neighbor with a strictly smaller
    // rank, and roots (rank 0) must be endpoints (at most one same-region
    // neighbor). Given the branching constraint above, a connected
    // component admits such ranks iff it is a path (or a single cell), so
    // loops are excluded while every path remains representable.
    let max_rank = (h * w - 1) as i32;
    let rank = &solver.int_var_2d((h, w), 0, max_rank);
    for y in 0..h {
        for x in 0..w {
            let rank_here = rank.at((y, x));

            // A root must be an endpoint.
            solver.add_expr(rank_here.eq(0).imp(deg[y][x].le(1)));

            // A non-root cell must have a same-region neighbor with a
            // smaller rank.
            let mut terms = vec![];
            for &(dy, dx) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let y2 = y as i32 + dy;
                let x2 = x as i32 + dx;
                if 0 <= y2 && y2 < h as i32 && 0 <= x2 && x2 < w as i32 {
                    let (y2, x2) = (y2 as usize, x2 as usize);
                    terms.push(
                        num.at((y2, x2)).eq(num.at((y, x)))
                            & rank.at((y2, x2)).lt(rank_here.clone()),
                    );
                }
            }
            solver.add_expr(rank_here.ge(1).imp(any(terms)));
        }
    }

    solver
        .irrefutable_facts()
        .map(|f| (f.get(num), f.get(&is_border)))
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
    problem_to_url(combinator(), "snakyfillomino", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["snakyfillomino"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        vec![
            vec![Some(9), Some(1), None, None, None],
            vec![None, None, None, Some(3), None],
            vec![Some(4), None, Some(1), None, None],
            vec![None, Some(2), Some(4), None, None],
            vec![None, None, None, None, None],
        ]
    }

    #[test]
    fn test_snaky_fillomino_problem() {
        let problem = problem_for_tests();
        let ans = solve_snaky_fillomino(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        let expected = crate::util::tests::to_option_2d([
            [9, 1, 9, 9, 9],
            [9, 9, 9, 3, 9],
            [4, 4, 1, 3, 9],
            [4, 2, 4, 3, 1],
            [4, 2, 4, 4, 4],
        ]);
        assert_eq!(ans.0, expected);
    }

    #[test]
    fn test_snaky_fillomino_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?snakyfillomino/5/5/91l3g4g1i24m";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
