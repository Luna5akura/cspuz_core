use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::pills;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = pills::deserialize_problem(url).ok_or("invalid url")?;
    let answer = pills::solve_pills(&problem).ok_or("no answer")?;

    let height = answer.len();
    let width = answer.first().map(|row| row.len()).unwrap_or(0);
    if height == 0
        || width == 0
        || answer.iter().any(|row| row.len() != width)
        || problem.dots.len() != height
        || problem.dots.iter().any(|row| row.len() != width)
    {
        return Err("invalid answer");
    }

    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&answer));
    for y in 0..height {
        for x in 0..width {
            if let Some(n) = answer[y][x] {
                if n > 0 {
                    board.push(Item::cell(y, x, "green", ItemKind::Num(n)));
                }
            }
        }
    }

    Ok(board)
}
