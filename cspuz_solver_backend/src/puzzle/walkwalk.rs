use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::walkwalk;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let (borders, clues) = walkwalk::deserialize_problem(url).ok_or("invalid url")?;
    let (is_line, is_passed) = walkwalk::solve_walkwalk(&borders, &clues).ok_or("no answer")?;

    let height = borders.vertical.len();
    let width = borders.vertical[0].len() + 1;
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&(&is_line, &is_passed)));
    board.add_borders(&borders, "black");

    for y in 0..height {
        for x in 0..width {
            if is_passed[y][x] == Some(true) {
                board.push(Item::cell(y, x, "green", ItemKind::Dot));
            }
            if let Some(n) = clues[y][x] {
                if n >= 0 {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                } else {
                    board.push(Item::cell(y, x, "black", ItemKind::Text("?")));
                }
            }
        }
    }

    board.add_lines_irrefutable_facts(&is_line, "green", None);
    Ok(board)
}
