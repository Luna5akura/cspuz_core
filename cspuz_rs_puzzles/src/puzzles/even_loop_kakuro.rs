use crate::util;
use cspuz_rs::complex_constraints::sum_all_different;
use cspuz_rs::graph;
use cspuz_rs::serializer::{problem_to_url_with_context, url_to_problem, Context};
use cspuz_rs::solver::{any, FromModel, IntVarArray1D, Solver};

use super::kakuro::{self, KakuroClue, Problem};

pub fn add_even_loop_constraints(
    solver: &mut Solver,
    numbers: &cspuz_rs::solver::IntVarArray2D,
    is_line: &graph::BoolGridEdges,
    h: usize,
    w: usize,
) {
    // Frame edges (on the outer boundary) are never used by the loop.
    for y in 0..=h {
        for x in 0..w {
            if y == 0 || y == h {
                solver.add_expr(!is_line.horizontal.at((y, x)));
            }
        }
    }
    for y in 0..h {
        for x in 0..=w {
            if x == 0 || x == w {
                solver.add_expr(!is_line.vertical.at((y, x)));
            }
        }
    }

    {
        let (edge_vars, _) = is_line.representation();
        let mut cell_graph = graph::Graph::new(h * w);
        for y in 0..=h {
            for x in 0..=w {
                if y < h {
                    if x > 0 && x < w {
                        cell_graph.add_edge(y * w + x - 1, y * w + x);
                    } else if x == 0 {
                        cell_graph.add_edge(y * w, y * w);
                    } else {
                        cell_graph.add_edge(y * w + w - 1, y * w + w - 1);
                    }
                }
                if x < w {
                    if y > 0 && y < h {
                        cell_graph.add_edge((y - 1) * w + x, y * w + x);
                    } else if y == 0 {
                        cell_graph.add_edge(x, x);
                    } else {
                        cell_graph.add_edge((h - 1) * w + x, (h - 1) * w + x);
                    }
                }
            }
        }
        graph::active_edges_single_cycle(solver, edge_vars, &cell_graph);
    }

    for y in 0..h {
        for x in 0..w {
            let even = any(vec![
                numbers.at((y, x)).eq(2),
                numbers.at((y, x)).eq(4),
                numbers.at((y, x)).eq(6),
                numbers.at((y, x)).eq(8),
            ]);
            let deg = is_line.cell_neighbors((y, x)).count_true();
            solver.add_expr(deg.eq(even.ite(2, 0)));
        }
    }
}

pub fn solve_even_loop_kakuro(
    clues: &[Vec<Option<KakuroClue>>],
) -> Option<(Vec<Vec<Option<i32>>>, graph::BoolGridEdgesIrrefutableFacts)> {
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

    let mut add_constraints = |cells: IntVarArray1D, clue: Option<i32>| -> bool {
        if let Some(n) = clue {
            sum_all_different(&mut solver, cells, n, 1, 9, None)
        } else {
            solver.all_different(&cells);
            cells.len() <= 9
        }
    };

    for y in 0..h {
        for x in 0..w {
            if let Some(clue) = clues[y][x] {
                let mut y2 = y + 1;
                while y2 < h && clues[y2][x].is_none() {
                    y2 += 1;
                }
                if y2 - y >= 2 {
                    if !add_constraints(numbers.slice_fixed_x(((y + 1)..y2, x)), clue.down) {
                        return None;
                    }
                }

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

    // Even Loop rule: a single, non-branching, non-crossing closed loop can
    // be drawn through every white cell that contains an even digit.
    let is_line = &graph::BoolGridEdges::new(&mut solver, (h, w));
    solver.add_answer_key_bool(&is_line.horizontal);
    solver.add_answer_key_bool(&is_line.vertical);
    add_even_loop_constraints(&mut solver, numbers, is_line, h, w);

    solver
        .irrefutable_facts()
        .map(|f| (f.get(numbers), f.get(is_line)))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = util::infer_shape(problem);
    problem_to_url_with_context(
        kakuro::combinator(),
        "evenloopkakuro",
        kakuro::problem_to_intermediate(problem)?,
        &Context::sized(h - 1, w - 1),
    )
}

#[cfg(test)]
fn pinned_solution_is_valid(problem: &Problem, intended: &[[i32; 5]; 5]) -> bool {
    let (h, w) = util::infer_shape(problem);
    let mut solver = Solver::new();
    let numbers = &solver.int_var_2d((h, w), 0, 9);
    solver.add_answer_key_int(numbers);
    for y in 0..h {
        for x in 0..w {
            solver.add_expr(numbers.at((y, x)).eq(intended[y][x]));
        }
    }
    let is_line = &graph::BoolGridEdges::new(&mut solver, (h, w));
    solver.add_answer_key_bool(&is_line.horizontal);
    solver.add_answer_key_bool(&is_line.vertical);
    add_even_loop_constraints(&mut solver, numbers, is_line, h, w);
    solver.solve().is_some()
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let (intermediate_grid, rem_seq) =
        url_to_problem(kakuro::combinator(), &["evenloopkakuro"], url)?;
    kakuro::problem_from_intermediate(intermediate_grid, rem_seq)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        // 3x3 playable area. The intended answer:
        //   2 4 6
        //   8 3 2
        //   4 6 8
        // The even digits form a single loop around the center.
        let mut ret = vec![vec![None; 5]; 5];
        for y in 0..5 {
            for x in 0..5 {
                if x == 0 || y == 0 || x == 4 || y == 4 {
                    ret[y][x] = Some(KakuroClue {
                        down: None,
                        right: None,
                    });
                }
            }
        }
        ret[0][1] = Some(KakuroClue {
            down: Some(14),
            right: None,
        });
        ret[0][2] = Some(KakuroClue {
            down: Some(13),
            right: None,
        });
        ret[0][3] = Some(KakuroClue {
            down: Some(16),
            right: None,
        });
        ret[1][0] = Some(KakuroClue {
            down: None,
            right: Some(12),
        });
        ret[2][0] = Some(KakuroClue {
            down: None,
            right: Some(13),
        });
        ret[3][0] = Some(KakuroClue {
            down: None,
            right: Some(18),
        });
        ret
    }

    #[test]
    fn test_even_loop_kakuro_problem() {
        // The puzzle is solvable, the clue cells are fixed to 0, and the
        // intended answer is a valid model of the constraints.
        let problem = problem_for_tests();
        let ans = solve_even_loop_kakuro(&problem);
        assert!(ans.is_some());
        let (numbers, _lines) = ans.unwrap();
        for y in 0..5 {
            for x in 0..5 {
                if x == 0 || y == 0 || x == 4 || y == 4 {
                    assert_eq!(numbers[y][x], Some(0));
                }
            }
        }

        // Verify the intended solution satisfies the constraints: pin it and
        // check that the solver accepts it.
        let intended = [
            [0, 0, 0, 0, 0],
            [0, 2, 4, 6, 0],
            [0, 8, 3, 2, 0],
            [0, 4, 6, 8, 0],
            [0, 0, 0, 0, 0],
        ];
        assert!(pinned_solution_is_valid(&problem_for_tests(), &intended));
    }

    #[test]
    fn test_even_loop_kakuro_single_isolated_even_cell() {
        // A single even cell surrounded by odd cells cannot host a closed
        // loop, so the constraints must be unsatisfiable.
        let h = 5usize;
        let w = 5usize;
        let mut solver = Solver::new();
        let numbers = &solver.int_var_2d((h, w), 0, 9);
        solver.add_answer_key_int(numbers);
        for y in 0..h {
            for x in 0..w {
                let v = if y >= 1 && y <= 3 && x >= 1 && x <= 3 {
                    if y == 2 && x == 2 {
                        2
                    } else {
                        1
                    }
                } else {
                    0
                };
                solver.add_expr(numbers.at((y, x)).eq(v));
            }
        }
        let is_line = &graph::BoolGridEdges::new(&mut solver, (h, w));
        solver.add_answer_key_bool(&is_line.horizontal);
        solver.add_answer_key_bool(&is_line.vertical);
        add_even_loop_constraints(&mut solver, numbers, is_line, h, w);
        assert!(!solver.solve().is_some());
    }

    #[test]
    fn test_even_loop_kakuro_concave_loop() {
        // A concave (staircase-shaped) loop must be accepted. The loop only
        // needs to be a single closed loop without branching or crossing;
        // convexity is not required.
        //
        //   2 4 6 8
        //   8 1 3 6
        //   6 9 2 4
        //   4 2 8 1
        let h = 6usize;
        let w = 6usize;
        let mut solver = Solver::new();
        let numbers = &solver.int_var_2d((h, w), 0, 9);
        solver.add_answer_key_int(numbers);
        let answer = [
            [0, 0, 0, 0, 0, 0],
            [0, 2, 4, 6, 8, 0],
            [0, 8, 1, 3, 6, 0],
            [0, 6, 9, 2, 4, 0],
            [0, 4, 2, 8, 1, 0],
            [0, 0, 0, 0, 0, 0],
        ];
        for y in 0..h {
            for x in 0..w {
                solver.add_expr(numbers.at((y, x)).eq(answer[y][x]));
            }
        }
        let is_line = &graph::BoolGridEdges::new(&mut solver, (h, w));
        solver.add_answer_key_bool(&is_line.horizontal);
        solver.add_answer_key_bool(&is_line.vertical);
        add_even_loop_constraints(&mut solver, numbers, is_line, h, w);
        let model = solver.solve();
        assert!(model.is_some());
        let lines = is_line.from_model(&model.unwrap());

        // The staircase cycle: 12 edges around the concave shape
        //   (0,0)-(0,1)-(0,2)-(0,3)-(1,3)-(2,3)-(2,2)
        //        -(3,2)-(3,1)-(3,0)-(2,0)-(1,0)-(0,0)
        let expected: [(usize, usize); 6] = [
            // (y, x) in playable coords: horizontal edges
            (2, 1),
            (2, 4),
            (3, 1),
            (3, 4),
            (4, 1),
            (4, 3),
        ];
        for (y, x) in expected.iter() {
            assert!(lines.horizontal[*y][*x]);
        }
        for (y, x) in [(1, 2), (1, 3), (1, 4), (3, 4), (4, 2), (4, 3)].iter() {
            assert!(lines.vertical[*y][*x]);
        }
        // every other interior edge is off
        let mut total = 0;
        for y in 1..h {
            for x in 1..w {
                if lines.horizontal[y][x] {
                    total += 1;
                }
                if lines.vertical[y][x] {
                    total += 1;
                }
            }
        }
        assert_eq!(total, 12);
    }

    #[test]
    fn test_even_loop_kakuro_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?evenloopkakuro/4/4/m--m--m----------edgcdi";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}

#[cfg(test)]
mod unsat_tests {
    use super::*;

    #[test]
    fn test_even_loop_kakuro_unsat_topright() {
        // This puzzle is unsolvable:
        // - The right run (0,5),(0,6),(0,7) sums to 7 with (0,7)=1 forced,
        //   so (0,5),(0,6) = {2,4} are even and on the loop.
        // - Their only loop exits force (1,5),(1,6) to be even as well.
        // - The left run (0,0)..(0,3) sums to 23 (odd), so it cannot be all
        //   odd (even sum) and cannot have a single even (that cell would
        //   have degree at most 1), forcing three evens whose loop must
        //   connect to the top-right block through the rest of the board.
        // - Every run may contain at most 4 even digits (2,4,6,8 without
        //   repetition), and the bottom-left corner ends up unable to reach
        //   degree 2 while the wall at (7,3) blocks the bottom row.
        let url = "https://puzz.link/p?evenloopkakuro/8/8/n-7t.zzu.n------1D-------/";
        let problem = deserialize_problem(url).expect("deserialize");
        assert!(solve_even_loop_kakuro(&problem).is_none());
    }
}
