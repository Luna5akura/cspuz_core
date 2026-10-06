use crate::util;
use cspuz_rs::graph::{self, GridEdges};
use cspuz_rs::serializer::{
    problem_to_url, url_to_problem, Choice, Combinator, Dict, Grid, HexInt, Optionalize, Spaces,
};
use cspuz_rs::solver::{count_true, sum, Solver};

/// Hashi Tree: Hashiwokakero with the additional rule that the network of
/// bridges must not contain a loop.  Two parallel bridges between the same
/// pair of islands are treated as a single connection, so a connected bridge
/// network without loops is a tree: the number of island-to-island
/// connections equals the number of islands minus one.
pub fn solve_hashi_tree(clues: &[Vec<Option<i32>>]) -> Option<GridEdges<Vec<Vec<Option<i32>>>>> {
    if clues.is_empty() || clues[0].is_empty() {
        return None;
    }
    let (h, w) = util::infer_shape(clues);
    if clues.iter().any(|row| row.len() != w) {
        return None;
    }
    let island_count = clues.iter().flatten().filter(|c| c.is_some()).count();
    if island_count == 0 {
        return None;
    }

    let mut solver = Solver::new();
    let answer_horizontal = &solver.int_var_2d((h, w - 1), 0, 2);
    let answer_vertical = &solver.int_var_2d((h - 1, w), 0, 2);
    solver.add_answer_key_int(answer_horizontal);
    solver.add_answer_key_int(answer_vertical);

    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                let mut deg = vec![];
                if y > 0 {
                    deg.push(answer_vertical.at((y - 1, x)));
                }
                if y < h - 1 {
                    deg.push(answer_vertical.at((y, x)));
                }
                if x > 0 {
                    deg.push(answer_horizontal.at((y, x - 1)));
                }
                if x < w - 1 {
                    deg.push(answer_horizontal.at((y, x)));
                }

                if n >= 0 {
                    solver.add_expr(sum(deg).eq(n));
                } else {
                    solver.add_expr(sum(deg).gt(0));
                }
            } else {
                if y == 0 {
                    if h > 1 {
                        solver.add_expr(answer_vertical.at((y, x)).eq(0));
                    }
                } else if y == h - 1 {
                    solver.add_expr(answer_vertical.at((y - 1, x)).eq(0));
                } else {
                    solver.add_expr(
                        answer_vertical
                            .at((y - 1, x))
                            .eq(answer_vertical.at((y, x))),
                    );
                }
                if x == 0 {
                    if w > 1 {
                        solver.add_expr(answer_horizontal.at((y, x)).eq(0));
                    }
                } else if x == w - 1 {
                    solver.add_expr(answer_horizontal.at((y, x - 1)).eq(0));
                } else {
                    solver.add_expr(
                        answer_horizontal
                            .at((y, x - 1))
                            .eq(answer_horizontal.at((y, x))),
                    );
                }

                if 0 < y && y < h - 1 && 0 < x && x < w - 1 {
                    solver.add_expr(
                        !(answer_horizontal.at((y, x - 1)).gt(0)
                            & answer_vertical.at((y - 1, x)).gt(0)),
                    );
                }
            }
        }
    }

    let is_connected = &graph::BoolGridEdges::new(&mut solver, (h - 1, w - 1));
    solver.add_expr(is_connected.horizontal.iff(answer_horizontal.gt(0)));
    solver.add_expr(is_connected.vertical.iff(answer_vertical.gt(0)));

    let (edges, g) = is_connected.representation();
    // All bridges form one connected network (inherited from Hashi).
    graph::active_vertices_connected(&mut solver, edges, &g.line_graph());
    // A connected bridge network without loops is a tree: the number of
    // island-to-island connections equals the number of islands minus one.
    // Count connections between horizontally/vertically adjacent islands
    // (a bridge spanning several cells still counts as one connection).
    let mut connections = vec![];
    for y in 0..h {
        let mut prev_x = None;
        for x in 0..w {
            if clues[y][x].is_some() {
                if let Some(px) = prev_x {
                    connections.push(answer_horizontal.at((y, px)).gt(0));
                }
                prev_x = Some(x);
            }
        }
    }
    for x in 0..w {
        let mut prev_y = None;
        for y in 0..h {
            if clues[y][x].is_some() {
                if let Some(py) = prev_y {
                    connections.push(answer_vertical.at((py, x)).gt(0));
                }
                prev_y = Some(y);
            }
        }
    }
    solver.add_expr(count_true(connections).eq((island_count - 1) as i32));

    solver.irrefutable_facts().map(|f| GridEdges {
        horizontal: f.get(answer_horizontal),
        vertical: f.get(answer_vertical),
    })
}

pub fn enumerate_answers_hashi_tree(
    clues: &[Vec<Option<i32>>],
    num_max_answers: usize,
) -> Vec<GridEdges<Vec<Vec<i32>>>> {
    if clues.is_empty() || clues[0].is_empty() {
        return vec![];
    }
    let (h, w) = util::infer_shape(clues);

    let mut solver = Solver::new();
    let answer_horizontal = &solver.int_var_2d((h, w - 1), 0, 2);
    let answer_vertical = &solver.int_var_2d((h - 1, w), 0, 2);
    solver.add_answer_key_int(answer_horizontal);
    solver.add_answer_key_int(answer_vertical);

    let island_count = clues.iter().flatten().filter(|c| c.is_some()).count();

    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                let mut deg = vec![];
                if y > 0 {
                    deg.push(answer_vertical.at((y - 1, x)));
                }
                if y < h - 1 {
                    deg.push(answer_vertical.at((y, x)));
                }
                if x > 0 {
                    deg.push(answer_horizontal.at((y, x - 1)));
                }
                if x < w - 1 {
                    deg.push(answer_horizontal.at((y, x)));
                }
                if n >= 0 {
                    solver.add_expr(sum(deg).eq(n));
                } else {
                    solver.add_expr(sum(deg).gt(0));
                }
            } else {
                if y == 0 {
                    if h > 1 {
                        solver.add_expr(answer_vertical.at((y, x)).eq(0));
                    }
                } else if y == h - 1 {
                    solver.add_expr(answer_vertical.at((y - 1, x)).eq(0));
                } else {
                    solver.add_expr(
                        answer_vertical
                            .at((y - 1, x))
                            .eq(answer_vertical.at((y, x))),
                    );
                }
                if x == 0 {
                    if w > 1 {
                        solver.add_expr(answer_horizontal.at((y, x)).eq(0));
                    }
                } else if x == w - 1 {
                    solver.add_expr(answer_horizontal.at((y, x - 1)).eq(0));
                } else {
                    solver.add_expr(
                        answer_horizontal
                            .at((y, x - 1))
                            .eq(answer_horizontal.at((y, x))),
                    );
                }
                if 0 < y && y < h - 1 && 0 < x && x < w - 1 {
                    solver.add_expr(
                        !(answer_horizontal.at((y, x - 1)).gt(0)
                            & answer_vertical.at((y - 1, x)).gt(0)),
                    );
                }
            }
        }
    }

    let is_connected = &graph::BoolGridEdges::new(&mut solver, (h - 1, w - 1));
    solver.add_expr(is_connected.horizontal.iff(answer_horizontal.gt(0)));
    solver.add_expr(is_connected.vertical.iff(answer_vertical.gt(0)));

    let (edges, g) = is_connected.representation();
    graph::active_vertices_connected(&mut solver, edges, &g.line_graph());
    // A connected bridge network without loops is a tree: the number of
    // island-to-island connections equals the number of islands minus one.
    // Count connections between horizontally/vertically adjacent islands
    // (a bridge spanning several cells still counts as one connection).
    if island_count > 0 {
        let mut connections = vec![];
        for y in 0..h {
            let mut prev_x = None;
            for x in 0..w {
                if clues[y][x].is_some() {
                    if let Some(px) = prev_x {
                        connections.push(answer_horizontal.at((y, px)).gt(0));
                    }
                    prev_x = Some(x);
                }
            }
        }
        for x in 0..w {
            let mut prev_y = None;
            for y in 0..h {
                if clues[y][x].is_some() {
                    if let Some(py) = prev_y {
                        connections.push(answer_vertical.at((py, x)).gt(0));
                    }
                    prev_y = Some(y);
                }
            }
        }
        solver.add_expr(count_true(connections).eq((island_count - 1) as i32));
    }

    solver
        .answer_iter()
        .take(num_max_answers)
        .map(|f| GridEdges {
            horizontal: f.get_unwrap(answer_horizontal),
            vertical: f.get_unwrap(answer_vertical),
        })
        .collect()
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
    if problem.is_empty() || problem[0].is_empty() {
        return None;
    }
    let (h, w) = util::infer_shape(problem);
    if problem.iter().any(|row| row.len() != w) {
        return None;
    }
    let _ = h;
    problem_to_url(combinator(), "hashitree", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["hashitree"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clues(problem: &[Vec<i32>]) -> Problem {
        problem
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&n| {
                        if n == i32::MIN {
                            None
                        } else {
                            Some(n)
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn test_hashi_tree_problem() {
        // 1-2-1 chain in a row: the only tree arrangement connects all three
        // islands with single bridges.
        let problem = clues(&[vec![1, i32::MIN, 2, i32::MIN, 1]]);
        let answers = enumerate_answers_hashi_tree(&problem, 8);
        assert_eq!(answers.len(), 1);
        let ans = &answers[0];
        // 桥跨越空格时每个 border 段都为 1
        assert_eq!(ans.horizontal, vec![vec![1, 1, 1, 1]]);
        assert!(ans.vertical.is_empty());
    }

    #[test]
    fn rejects_loop() {
        // Four corner 2s in a 3x3 grid: in plain Hashi the outer cycle of
        // bridges is valid, but Hashi Tree forbids loops.
        let problem = clues(&[
            vec![2, i32::MIN, 2],
            vec![i32::MIN, i32::MIN, i32::MIN],
            vec![2, i32::MIN, 2],
        ]);
        let answers = enumerate_answers_hashi_tree(&problem, 8);
        // Any connected solution needs at least 4 connections but a tree on
        // 4 islands has at most 3, so no solution exists.
        assert_eq!(answers.len(), 0);
    }

    #[test]
    fn test_hashi_tree_serializer() {
        let problem = vec![
            vec![Some(3), None, Some(1)],
            vec![None, Some(-1), None],
            vec![Some(2), None, Some(2)],
        ];
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn rejects_empty_or_ragged_grids() {
        let empty: Problem = vec![];
        assert!(solve_hashi_tree(&empty).is_none());
        assert!(serialize_problem(&empty).is_none());
        let ragged = vec![vec![Some(1), None], vec![None]];
        assert!(serialize_problem(&ragged).is_none());
    }
}
