use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::shape_minesweeper;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let (clues, shapes) = shape_minesweeper::deserialize_problem(url).ok_or("invalid url")?;
    let answer = shape_minesweeper::solve_shape_minesweeper(&clues, &shapes).ok_or("no answer")?;
    let height = clues.len();
    let width = clues.first().map(|r| r.len()).unwrap_or(0);
    if height == 0 || clues.iter().any(|r| r.len() != width) || answer.len() != height {
        return Err("invalid answer");
    }
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&answer));
    for y in 0..height {
        for x in 0..width {
            if let Some(n) = clues[y][x] {
                board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
            } else if let Some(v) = answer[y][x] {
                board.push(Item::cell(
                    y,
                    x,
                    "green",
                    if v { ItemKind::Fill } else { ItemKind::Dot },
                ));
            }
        }
    }
    Ok(board)
}
