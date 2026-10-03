use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::even_loop_kakuro;
use cspuz_rs_puzzles::puzzles::kakuro::KakuroClue;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = even_loop_kakuro::deserialize_problem(url).ok_or("invalid url")?;
    let (numbers, lines) = even_loop_kakuro::solve_even_loop_kakuro(&problem).ok_or("no answer")?;

    let height = numbers.len();
    let width = numbers[0].len();
    let mut board = Board::new(
        BoardKind::OuterGrid,
        height,
        width,
        is_unique(&(&numbers, &lines)),
    );

    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = problem[y][x] {
                if let Some(n) = clue.down {
                    board.push(Item::cell(y, x, "black", ItemKind::NumLowerRight(n)));
                }
                if let Some(n) = clue.right {
                    board.push(Item::cell(y, x, "black", ItemKind::NumUpperLeft(n)));
                }
                board.push(Item::cell(y, x, "black", ItemKind::Block));
            } else if let Some(n) = numbers[y][x] {
                board.push(Item::cell(y, x, "green", ItemKind::Num(n)));
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            if y < height - 1 {
                if let Some(b) = lines.horizontal[y + 1][x] {
                    board.push(Item {
                        y: y * 2 + 2,
                        x: x * 2 + 1,
                        color: "green",
                        kind: if b {
                            ItemKind::BoldWall
                        } else {
                            ItemKind::Cross
                        },
                    });
                }
            }
            if x < width - 1 {
                if let Some(b) = lines.vertical[y][x + 1] {
                    board.push(Item {
                        y: y * 2 + 1,
                        x: x * 2 + 2,
                        color: "green",
                        kind: if b {
                            ItemKind::BoldWall
                        } else {
                            ItemKind::Cross
                        },
                    });
                }
            }
        }
    }

    Ok(board)
}
