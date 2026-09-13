use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{problem_to_url, url_to_problem, Choice, Combinator, Dict, Grid, NumSpaces, Spaces};
use cspuz_rs::solver::Solver;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark { None, Sheep, Wolf }

pub type Problem = (Vec<Vec<Option<i32>>>, Vec<Vec<Mark>>);

pub fn solve_wolves_and_sheep(clues: &[Vec<Option<i32>>], marks: &[Vec<Mark>]) -> Option<graph::BoolGridEdgesIrrefutableFacts> {
    let (h, w) = util::infer_shape(clues);
    let mut solver = Solver::new();
    let edges = &graph::BoolGridEdges::new(&mut solver, (h, w));
    let inside = solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(&edges.horizontal);
    solver.add_answer_key_bool(&edges.vertical);
    graph::single_cycle_grid_edges(&mut solver, edges);

    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                if (0..=4).contains(&n) { solver.add_expr(edges.cell_neighbors((y, x)).count_true().eq(n)); }
            }
            match marks.get(y).and_then(|r| r.get(x)).copied().unwrap_or(Mark::None) {
                Mark::Sheep => solver.add_expr(inside.at((y, x)).iff(true)),
                Mark::Wolf => solver.add_expr(inside.at((y, x)).iff(false)),
                Mark::None => {}
            }
            // A fence separates inside from outside; boundary edges separate from outside.
            solver.add_expr(edges.vertical.at((y, x)).iff(if x == 0 { inside.at((y, x)).expr() } else { (&inside.at((y, x)) ^ &inside.at((y, x - 1))) }));
            solver.add_expr(edges.vertical.at((y, x + 1)).iff(if x + 1 == w { inside.at((y, x)).expr() } else { (&inside.at((y, x)) ^ &inside.at((y, x + 1))) }));
            solver.add_expr(edges.horizontal.at((y, x)).iff(if y == 0 { inside.at((y, x)).expr() } else { (&inside.at((y, x)) ^ &inside.at((y - 1, x))) }));
            solver.add_expr(edges.horizontal.at((y + 1, x)).iff(if y + 1 == h { inside.at((y, x)).expr() } else { (&inside.at((y, x)) ^ &inside.at((y + 1, x))) }));
        }
    }
    solver.irrefutable_facts().map(|f| f.get(edges))
}

pub(crate) fn combinator() -> impl Combinator<Vec<Vec<Option<i32>>>> {
    Grid::new(Choice::new(vec![
        Box::new(NumSpaces::new(6, 2)),
        Box::new(Spaces::new(None, 'g')),
        Box::new(Dict::new(None, ".")),
    ]))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let raw = url_to_problem(combinator(), &["wolvesandsheepfences", "wolves-and-sheep-fences"], url)?;
    let marks = raw.iter().map(|r| r.iter().map(|v| match v { Some(5) => Mark::Sheep, Some(6) => Mark::Wolf, _ => Mark::None }).collect()).collect();
    let clues = raw.into_iter().map(|r| r.into_iter().map(|v| v.filter(|n| *n <= 4)).collect()).collect();
    Some((clues, marks))
}
pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (clues, marks) = problem;
    let raw = clues.iter().zip(marks).map(|(cr,mr)| cr.iter().zip(mr).map(|(c,m)| match m { Mark::Sheep => Some(5), Mark::Wolf => Some(6), Mark::None => *c }).collect()).collect();
    problem_to_url(combinator(), "wolvesandsheepfences", raw)
}
