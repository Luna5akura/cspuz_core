use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::sky_neighbor;

/// Convert a Sky Neighbors answer to an 11x11 outer-grid description.  The
/// generic solver UI uses non-negative coordinates, so the virtual pzpr
/// coordinates are shifted by one cell here (the UI shifts them back).
pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = sky_neighbor::deserialize_problem(url).ok_or("invalid url")?;
    let (inner, outer) = sky_neighbor::solve(&problem).ok_or("no answer")?;
    let mut board = Board::new(
        BoardKind::OuterGrid,
        sky_neighbor::SKY_NEIGHBOR_SIZE + 2,
        sky_neighbor::SKY_NEIGHBOR_SIZE + 2,
        is_unique(&(&inner, &outer)),
    );

    let push_num = |board: &mut Board, y: usize, x: usize, color: &'static str, n: i32| {
        board.push(Item {
            y,
            x,
            color,
            kind: ItemKind::Num(n),
        });
    };

    for y in 0..9 {
        for x in 0..9 {
            let gy = y + 1;
            let gx = x + 1;
            if problem.inner_outlined[y][x] {
                board.push(Item {
                    y: gy * 2 + 1,
                    x: gx * 2 + 1,
                    color: "black",
                    kind: ItemKind::Square,
                });
            }
            if let Some(n) = problem.inner_givens[y][x] {
                push_num(&mut board, gy * 2 + 1, gx * 2 + 1, "black", n);
            } else if let Some(n) = inner[y][x] {
                push_num(&mut board, gy * 2 + 1, gx * 2 + 1, "green", n);
            }
        }
    }

    for i in 0..36 {
        let (y, x) = if i < 9 {
            (0, i + 1)
        } else if i < 18 {
            (10, i - 9 + 1)
        } else if i < 27 {
            (i - 18 + 1, 0)
        } else {
            (i - 27 + 1, 10)
        };
        if problem.outer_outlined[i] {
            board.push(Item {
                y: y * 2 + 1,
                x: x * 2 + 1,
                color: "black",
                kind: ItemKind::Square,
            });
        }
        if let Some(n) = problem.outer_givens[i] {
            push_num(&mut board, y * 2 + 1, x * 2 + 1, "black", n);
        } else if let Some(n) = outer[i] {
            push_num(&mut board, y * 2 + 1, x * 2 + 1, "green", n);
        }
    }

    Ok(board)
}
