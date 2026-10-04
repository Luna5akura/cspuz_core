use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::tridbchoco;

/// 盤面の形: 頂点が上向きの大きな正三角形 (solve側と同じ規則)。
/// 頂点の列apexは偶数、行数rowsは偶数に丸める。
fn tri_in_region(height: usize, width: usize, y: usize, x: usize) -> bool {
    let mut apex = (width - 1) / 2;
    if apex % 2 == 1 {
        apex -= 1;
    }
    let mut rows = height.min(apex + 1).min(width - apex);
    if rows % 2 == 1 {
        rows -= 1;
    }
    y < rows && x + y >= apex && x <= apex + y
}

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
            if !tri_in_region(height, width, y, x) {
                continue;
            }
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

    // 左右の境界線 (y,x)-(y,x+1) (両側のセルが盤内のときだけ実在)
    for y in 0..height {
        for x in 0..(width - 1) {
            if !tri_in_region(height, width, y, x)
                || !tri_in_region(height, width, y, x + 1)
            {
                continue;
            }
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
            if !tri_in_region(height, width, y, x)
                || !tri_in_region(height, width, y + 1, x)
            {
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
