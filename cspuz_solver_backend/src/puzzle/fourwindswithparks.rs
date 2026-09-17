use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::fourwindswithparks;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let clues = fourwindswithparks::deserialize_problem(url).ok_or("invalid url")?;
    let answer = fourwindswithparks::solve_fourwindswithparks(&clues).ok_or("no answer")?;
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
            } else {
                let kind = match answer[y][x] {
                    Some(1) => ItemKind::ArrowUp,
                    Some(2) => ItemKind::ArrowDown,
                    Some(3) => ItemKind::ArrowLeft,
                    Some(4) => ItemKind::ArrowRight,
                    _ => continue,
                };
                board.push(Item::cell(y, x, "green", kind));
            }
        }
    }
    Ok(board)
}
