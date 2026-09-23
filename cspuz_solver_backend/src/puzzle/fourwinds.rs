use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::fourwinds;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let clues = fourwinds::deserialize_problem(url).ok_or("invalid url")?;
    let answer = fourwinds::solve_fourwinds(&clues).ok_or("no answer")?;
    let height = clues.len();
    let width = clues.first().map(|row| row.len()).ok_or("invalid url")?;
    if height == 0 || width == 0 || clues.iter().any(|row| row.len() != width) {
        return Err("invalid url");
    }

    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&answer));
    for y in 0..height {
        for x in 0..width {
            if let Some(n) = clues[y][x] {
                board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
            } else if let Some(d) = answer[y][x] {
                let kind = match d {
                    1 => ItemKind::ArrowUp,
                    2 => ItemKind::ArrowDown,
                    3 => ItemKind::ArrowLeft,
                    4 => ItemKind::ArrowRight,
                    _ => continue,
                };
                board.push(Item::cell(y, x, "green", kind));
            }
        }
    }
    Ok(board)
}
