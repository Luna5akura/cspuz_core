use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{problem_to_url, url_to_problem, Combinator, Context, Grid};
use cspuz_rs::solver::Solver;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    None,
    Sheep,
    Wolf,
}

pub type Problem = (Vec<Vec<Option<i32>>>, Vec<Vec<Mark>>);

pub fn solve_wolves_and_sheep(
    clues: &[Vec<Option<i32>>],
    marks: &[Vec<Mark>],
) -> Option<graph::BoolGridEdgesIrrefutableFacts> {
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
                if (0..=4).contains(&n) {
                    solver.add_expr(edges.cell_neighbors((y, x)).count_true().eq(n));
                }
            }
            match marks
                .get(y)
                .and_then(|r| r.get(x))
                .copied()
                .unwrap_or(Mark::None)
            {
                Mark::Sheep => solver.add_expr(inside.at((y, x)).iff(true)),
                Mark::Wolf => solver.add_expr(inside.at((y, x)).iff(false)),
                Mark::None => {}
            }
            // A fence separates inside from outside; boundary edges separate from outside.
            solver.add_expr(edges.vertical.at((y, x)).iff(if x == 0 {
                inside.at((y, x)).expr()
            } else {
                (&inside.at((y, x)) ^ &inside.at((y, x - 1)))
            }));
            solver.add_expr(edges.vertical.at((y, x + 1)).iff(if x + 1 == w {
                inside.at((y, x)).expr()
            } else {
                (&inside.at((y, x)) ^ &inside.at((y, x + 1)))
            }));
            solver.add_expr(edges.horizontal.at((y, x)).iff(if y == 0 {
                inside.at((y, x)).expr()
            } else {
                (&inside.at((y, x)) ^ &inside.at((y - 1, x)))
            }));
            solver.add_expr(edges.horizontal.at((y + 1, x)).iff(if y + 1 == h {
                inside.at((y, x)).expr()
            } else {
                (&inside.at((y, x)) ^ &inside.at((y + 1, x)))
            }));
        }
    }
    solver.irrefutable_facts().map(|f| f.get(edges))
}

/// Pzpr's `decodeNumber10` format: decimal clues are literal, `.` is one
/// empty cell, and `a`..`z` encode runs of 1..26 empty cells.
struct Number10;

impl Combinator<Option<i32>> for Number10 {
    fn serialize(&self, _: &Context, input: &[Option<i32>]) -> Option<(usize, Vec<u8>)> {
        let first = input.first()?;
        match first {
            Some(n) if (0..=9).contains(n) => Some((1, vec![b'0' + *n as u8])),
            None => {
                let mut count = 0;
                while count < input.len() && input[count].is_none() && count < 26 {
                    count += 1;
                }
                if count == 0 {
                    None
                } else if count == 1 {
                    Some((1, vec![b'.']))
                } else {
                    Some((
                        count,
                        vec![cspuz_rs::serializer::to_base36((count + 9) as i32)],
                    ))
                }
            }
            _ => None,
        }
    }

    fn deserialize(&self, _: &Context, input: &[u8]) -> Option<(usize, Vec<Option<i32>>)> {
        let c = *input.first()?;
        match c {
            b'0'..=b'9' => Some((1, vec![Some((c - b'0') as i32)])),
            b'.' => Some((1, vec![None])),
            b'a'..=b'z' => {
                let count = (c - b'a' + 1) as usize;
                Some((1, vec![None; count]))
            }
            _ => None,
        }
    }
}

pub(crate) fn combinator() -> impl Combinator<Vec<Vec<Option<i32>>>> {
    Grid::new(Number10)
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let raw = url_to_problem(
        combinator(),
        &["wolvesandsheepfences", "wolves-and-sheep-fences"],
        url,
    )?;
    let marks = raw
        .iter()
        .map(|r| {
            r.iter()
                .map(|v| match v {
                    Some(5) => Mark::Sheep,
                    Some(6) => Mark::Wolf,
                    _ => Mark::None,
                })
                .collect()
        })
        .collect();
    let clues = raw
        .into_iter()
        .map(|r| r.into_iter().map(|v| v.filter(|n| *n <= 4)).collect())
        .collect();
    Some((clues, marks))
}
pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (clues, marks) = problem;
    let raw = clues
        .iter()
        .zip(marks)
        .map(|(cr, mr)| {
            cr.iter()
                .zip(mr)
                .map(|(c, m)| match m {
                    Mark::Sheep => Some(5),
                    Mark::Wolf => Some(6),
                    Mark::None => *c,
                })
                .collect()
        })
        .collect();
    problem_to_url(combinator(), "wolvesandsheepfences", raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_pzpr_number10_marks() {
        let (clues, marks) = deserialize_problem(
            "https://puzz.link/p?wolvesandsheepfences/5/5/2c5a2a5c2a136a3c6a3",
        )
        .unwrap();
        assert_eq!(clues[0], vec![Some(2), None, None, None, None]);
        assert_eq!(marks[0][4], Mark::Sheep);
        assert_eq!(marks[3][1], Mark::Wolf);
    }
}
