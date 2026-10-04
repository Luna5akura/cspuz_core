use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::tridbchoco;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let (color, num) = tridbchoco::deserialize_problem(url).ok_or("invalid url")?;
    let (hor, ver) = tridbchoco::solve_tridbchoco(&color, &num).ok_or("no answer")?;

    let height = num.len();
    let width = num[0].len();
    let mut board = Board::new(
        BoardKind::OuterGrid,
        height,
        width,
        is_unique(&(&hor, &ver)),
    );

    for y in 0..height {
        for x in 0..width {
            if color[y][x] == 1 {
                board.push(Item::cell(y, x, "#cccccc", ItemKind::Fill));
            }
            if let Some(n) = num[y][x] {
                if n == -1 {
                    board.push(Item::cell(y, x, "black", ItemKind::Text("?")));
                } else {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                }
            }
        }
    }

    // 左右の境界線 (y,x)-(y,x+1)
    for y in 0..height {
        for x in 0..(width - 1) {
            if let Some(b) = hor[y][x] {
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

    // 上下の境界線 (y,x)-(y+1,x) (上側のセルが△のときだけ実在)
    for y in 0..(height - 1) {
        for x in 0..width {
            if (x + y) % 2 != 0 {
                continue;
            }
            if let Some(b) = ver[y][x] {
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
    }

    Ok(board)
}
