use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::japanese_arrows;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let clues = japanese_arrows::deserialize_problem(url).ok_or("invalid url")?;
    let answer = japanese_arrows::solve_japanese_arrows(&clues).ok_or("no answer")?;
    let height = clues.len();
    let width = clues.first().map(|row| row.len()).ok_or("invalid url")?;
    if height == 0 || width == 0 || clues.iter().any(|row| row.len() != width) {
        return Err("invalid url");
    }

    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&answer));
    for y in 0..height {
        for x in 0..width {
            let clue = clues[y][x];
            if let Some(direction) = clue.direction {
                let kind = match direction {
                    1 => ItemKind::ArrowUp,
                    2 => ItemKind::ArrowDown,
                    3 => ItemKind::ArrowLeft,
                    4 => ItemKind::ArrowRight,
                    5 => ItemKind::ArrowUpLeft,
                    6 => ItemKind::ArrowUpRight,
                    7 => ItemKind::ArrowDownLeft,
                    8 => ItemKind::ArrowDownRight,
                    _ => return Err("invalid clue"),
                };
                board.push(Item::cell(y, x, "black", kind));
            }
            if let Some(count) = clue.count {
                // The arrow occupies the left side of the cell in this
                // variety; keep the clue number on the right as in the
                // printed competition puzzle.
                board.push(Item::cell(y, x, "black", ItemKind::NumUpperRight(count)));
            }
            if clue.count.is_none() {
                if let Some(n) = answer[y][x] {
                    // Japanese Arrows reserves the left side of each cell for
                    // its arrow.  NumUpperRight is interpreted by the pzpr
                    // variety as the right-side answer-number overlay.
                    board.push(Item::cell(y, x, "green", ItemKind::NumUpperRight(n)));
                }
            }
        }
    }
    Ok(board)
}
