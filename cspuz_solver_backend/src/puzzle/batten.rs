use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::{is_unique, Uniqueness};
use cspuz_rs_puzzles::puzzles::batten;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = batten::deserialize_problem(url).ok_or("invalid url")?;
    let (shaded, truncated) = batten::solve_batten(&problem).ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.1.len();
    let mut board = Board::new(
        BoardKind::Grid,
        height,
        width,
        if truncated {
            Uniqueness::NonUnique
        } else {
            is_unique(&shaded)
        },
    );

    for y in 0..height {
        for x in 0..width {
            if let Some(a) = shaded[y][x] {
                board.push(Item::cell(
                    y,
                    x,
                    "green",
                    if a { ItemKind::Fill } else { ItemKind::Dot },
                ));
            }
        }
    }

    Ok(board)
}
