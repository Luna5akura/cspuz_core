use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::gravel::{self, Circle};

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = gravel::deserialize_problem(url).ok_or("invalid url")?;
    let ans = gravel::solve_gravel(&problem).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(
        BoardKind::OuterGrid,
        height,
        width,
        is_unique(&(&ans.is_black, &ans.borders)),
    );

    for y in 0..height {
        for x in 0..width {
            let clue = problem[y][x];
            if clue.empty {
                board.push(Item::cell(y, x, "black", ItemKind::Cross));
                continue;
            }
            match clue.circle {
                Some(Circle::White) => board.push(Item::cell(y, x, "black", ItemKind::Circle)),
                Some(Circle::Black) => {
                    board.push(Item::cell(y, x, "black", ItemKind::FilledCircle))
                }
                None => {}
            }
            if let Some(n) = clue.number {
                if n == -2 {
                    board.push(Item::cell(y, x, "black", ItemKind::Text("?")));
                } else if n >= 0 {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                }
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            if let Some(is_black) = ans.is_black[y][x] {
                if !problem[y][x].empty {
                    board.push(Item::cell(
                        y,
                        x,
                        "green",
                        if is_black {
                            ItemKind::Block
                        } else {
                            ItemKind::Dot
                        },
                    ));
                }
            }
            if y + 1 < height {
                if let Some(b) = ans.borders.horizontal[y][x] {
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
            if x + 1 < width {
                if let Some(b) = ans.borders.vertical[y][x] {
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
