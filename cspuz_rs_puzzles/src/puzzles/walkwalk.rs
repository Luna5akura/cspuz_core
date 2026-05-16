use cspuz_rs::graph;
use cspuz_rs::serializer::{
    problem_to_url_with_context, url_to_problem, Choice, Combinator, Context, ContextBasedGrid,
    Dict, HexInt, Optionalize, Rooms, Size, Spaces, Tuple2,
};
use cspuz_rs::solver::{BoolVarArray2D, Solver};

pub type Problem = (graph::InnerGridEdges<Vec<Vec<bool>>>, Vec<Vec<Option<i32>>>);

#[derive(Clone, Copy)]
enum RoomEdgeKind {
    Horizontal(usize, usize),
    Vertical(usize, usize),
}

fn build_walkwalk_model(
    borders: &graph::InnerGridEdges<Vec<Vec<bool>>>,
    clues: &[Vec<Option<i32>>],
) -> (Solver<'static>, graph::BoolGridEdges, BoolVarArray2D) {
    let (h, w) = borders.base_shape();

    let mut solver = Solver::new();
    let is_line = graph::BoolGridEdges::new(&mut solver, (h - 1, w - 1));
    solver.add_answer_key_bool(&is_line.horizontal);
    solver.add_answer_key_bool(&is_line.vertical);

    let is_passed = graph::single_cycle_grid_edges(&mut solver, &is_line);
    solver.add_answer_key_bool(&is_passed);

    let rooms = graph::borders_to_rooms(borders);
    let mut room_id = vec![vec![usize::MAX; w]; h];
    let mut room_pos = vec![vec![usize::MAX; w]; h];
    for (i, room) in rooms.iter().enumerate() {
        for (j, &(y, x)) in room.iter().enumerate() {
            room_id[y][x] = i;
            room_pos[y][x] = j;
        }
    }

    let mut room_graphs = Vec::with_capacity(rooms.len());
    let mut room_edges = Vec::with_capacity(rooms.len());
    for (rid, room) in rooms.iter().enumerate() {
        let mut graph = graph::Graph::new(room.len());
        let mut edges = vec![];
        for &(y, x) in room {
            let u = room_pos[y][x];
            if y + 1 < h && room_id[y + 1][x] == rid {
                graph.add_edge(u, room_pos[y + 1][x]);
                edges.push(RoomEdgeKind::Vertical(y, x));
            }
            if x + 1 < w && room_id[y][x + 1] == rid {
                graph.add_edge(u, room_pos[y][x + 1]);
                edges.push(RoomEdgeKind::Horizontal(y, x));
            }
        }
        room_graphs.push(graph);
        room_edges.push(edges);
    }

    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                solver.add_expr(is_passed.at((y, x)));

                let rid = room_id[y][x];
                let clue_pos = room_pos[y][x];
                let segment = solver.bool_var_1d(rooms[rid].len());
                solver.add_expr(segment.at(clue_pos));

                for (i, &(yy, xx)) in rooms[rid].iter().enumerate() {
                    solver.add_expr(segment.at(i).imp(is_passed.at((yy, xx))));
                }

                if n >= 0 {
                    solver.add_expr(segment.count_true().eq(n));
                }

                let mut active_edges = vec![];
                for (edge_idx, edge_kind) in room_edges[rid].iter().enumerate() {
                    let (u, v) = room_graphs[rid][edge_idx];
                    let edge = match *edge_kind {
                        RoomEdgeKind::Horizontal(yy, xx) => is_line.horizontal.at((yy, xx)),
                        RoomEdgeKind::Vertical(yy, xx) => is_line.vertical.at((yy, xx)),
                    };
                    solver.add_expr((segment.at(u) & edge.clone()).imp(segment.at(v)));
                    solver.add_expr((segment.at(v) & edge.clone()).imp(segment.at(u)));
                    active_edges.push(edge);
                }
                graph::active_vertices_connected_via_active_edges(
                    &mut solver,
                    &segment,
                    &active_edges,
                    &room_graphs[rid],
                );
            }
        }
    }

    (solver, is_line, is_passed)
}

pub fn solve_walkwalk(
    borders: &graph::InnerGridEdges<Vec<Vec<bool>>>,
    clues: &[Vec<Option<i32>>],
) -> Option<(
    graph::BoolGridEdgesIrrefutableFacts,
    Vec<Vec<Option<bool>>>,
)> {
    let (solver, is_line, is_passed) = build_walkwalk_model(borders, clues);

    solver
        .irrefutable_facts()
        .map(|f| (f.get(&is_line), f.get(&is_passed)))
}

fn combinator() -> impl Combinator<Problem> {
    Size::new(Tuple2::new(
        Rooms,
        ContextBasedGrid::new(Choice::new(vec![
            Box::new(Optionalize::new(HexInt)),
            Box::new(Spaces::new(None, 'g')),
            Box::new(Dict::new(Some(-1), ".")),
        ])),
    ))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let height = problem.0.vertical.len();
    let width = problem.0.vertical[0].len() + 1;
    problem_to_url_with_context(
        combinator(),
        "walkwalk",
        problem.clone(),
        &Context::sized(height, width),
    )
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["walkwalk"], url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    fn problem_for_tests() -> Problem {
        let borders = graph::InnerGridEdges {
            horizontal: crate::util::tests::to_bool_2d([[0, 0, 0, 0], [1, 1, 1, 0], [0, 0, 0, 0]]),
            vertical: crate::util::tests::to_bool_2d([[0, 1, 0], [0, 1, 0], [0, 0, 1], [0, 0, 1]]),
        };
        let clues = vec![
            vec![None, Some(4), None, None],
            vec![Some(4), None, None, None],
            vec![None, None, None, None],
            vec![None, None, None, None],
        ];
        (borders, clues)
    }

    #[test]
    fn test_walkwalk_problem() {
        let (borders, clues) = problem_for_tests();
        let ans = solve_walkwalk(&borders, &clues);
        assert!(ans.is_some());
        let (line_facts, passed_facts) = ans.unwrap();

        assert_eq!(line_facts.horizontal[0][0], Some(true));
        assert_eq!(line_facts.vertical[0][0], Some(true));
        assert_eq!(passed_facts[0][0], Some(true));
        assert_eq!(passed_facts[0][1], Some(true));
        assert_eq!(passed_facts[1][0], Some(true));
    }

    #[test]
    fn test_walkwalk_single_clue_cell_is_passed() {
        let (borders, clues) =
            deserialize_problem("https://puzz.link/p?walkwalk/3/3/g0g01n").unwrap();
        let ans = solve_walkwalk(&borders, &clues).unwrap();

        assert_eq!(ans.1[0][0], Some(true));
    }

    #[test]
    fn test_walkwalk_top_left_room_bent_six_is_possible() {
        let (borders, clues) =
            deserialize_problem("https://puzz.link/p?walkwalk/5/5/48g000s06zj").unwrap();
        let (mut solver, is_line, _) = build_walkwalk_model(&borders, &clues);

        solver.add_expr(is_line.horizontal.at((0, 0)));
        solver.add_expr(is_line.horizontal.at((0, 1)));
        solver.add_expr(is_line.horizontal.at((0, 2)));
        solver.add_expr(is_line.horizontal.at((1, 0)));
        solver.add_expr(is_line.horizontal.at((1, 1)));
        solver.add_expr(is_line.horizontal.at((1, 2)));
        solver.add_expr(is_line.vertical.at((0, 0)));
        solver.add_expr(is_line.vertical.at((0, 3)));

        assert!(solver.solve().is_some());
    }

    #[test]
    fn test_walkwalk_bent_six_edges_are_not_forced_false() {
        let (borders, clues) =
            deserialize_problem("https://puzz.link/p?walkwalk/5/5/48g000s06zj").unwrap();
        let ans = solve_walkwalk(&borders, &clues).unwrap();

        assert_eq!(ans.0.horizontal[1][0], None);
        assert_eq!(ans.0.horizontal[1][1], None);
    }

    #[test]
    fn test_walkwalk_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?walkwalk/4/4/9281o0g4h4q";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
