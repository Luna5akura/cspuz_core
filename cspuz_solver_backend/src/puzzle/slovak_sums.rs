use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::slovak_sums;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let (numbers, clues) = slovak_sums::deserialize_problem(url).ok_or("invalid url")?;
    let answer = slovak_sums::solve_slovak_sums(&numbers, &clues).ok_or("no answer")?;

    let height = clues.len();
    let width = clues[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&answer));

    for y in 0..height {
        for x in 0..width {
            match clues[y][x] {
                Some(slovak_sums::SlovakSumsCell::Blocked) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Fill));
                }
                Some(slovak_sums::SlovakSumsCell::Clue(clue)) => {
                    if let Some(sum) = clue.sum {
                        board.push(Item::cell(y, x, "black", ItemKind::NumUpperLeft(sum)));
                    }
                    if let Some(count) = clue.count {
                        board.push(Item::cell(y, x, "black", ItemKind::NumLowerLeft(count)));
                    }
                }
                None => {
                    if let Some(value) = answer[y][x] {
                        board.push(Item::cell(
                            y,
                            x,
                            "green",
                            if value == 0 {
                                ItemKind::Cross
                            } else {
                                ItemKind::Num(value)
                            },
                        ));
                    }
                }
            }
        }
    }

    Ok(board)
}
