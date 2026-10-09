use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs::graph;
use cspuz_rs_puzzles::puzzles::balance_loop;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let (problem, opts) = balance_loop::deserialize_problem(url).ok_or("invalid url")?;
    let is_line = balance_loop::solve_balance_loop_with_options(&problem, opts).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&is_line));

    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = problem[y][x] {
                board.push(Item::cell(
                    y,
                    x,
                    "black",
                    if clue.white {
                        ItemKind::Circle
                    } else {
                        ItemKind::FilledCircle
                    },
                ));
                board.push(Item::cell(
                    y,
                    x,
                    "black",
                    match clue.num {
                        Some(n) => ItemKind::Num(n),
                        None => ItemKind::Text("?"),
                    },
                ));
            }
        }
    }

    board.add_lines_irrefutable_facts(&is_line, "green", None);

    Ok(board)
}
