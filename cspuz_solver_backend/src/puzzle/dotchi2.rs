use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::dotchi2;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = dotchi2::deserialize_problem(url).ok_or("invalid url")?;
    let (h_edges, v_edges, truncated) = dotchi2::solve_dotchi2(&problem).ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.0[0].len();
    let mut board = Board::new(
        BoardKind::Grid,
        height,
        width,
        if truncated {
            crate::uniqueness::Uniqueness::NonUnique
        } else {
            is_unique(&h_edges)
        },
    );

    for y in 0..height {
        for x in 0..width {
            if let Some(c) = problem.0[y][x] {
                if c == 1 {
                    board.push(Item::cell(y, x, "black", ItemKind::Circle));
                } else if c == 2 {
                    board.push(Item::cell(y, x, "black", ItemKind::FilledCircle));
                }
            }
            if x < width - 1 {
                if problem.1[y][x] {
                    board.push(Item {
                        y: y * 2 + 1,
                        x: x * 2 + 2,
                        color: "black",
                        kind: ItemKind::BoldWall,
                    });
                }
                if let Some(true) = h_edges[y][x] {
                    board.push(Item {
                        y: y * 2 + 1,
                        x: x * 2 + 2,
                        color: "green",
                        kind: ItemKind::Line,
                    });
                }
            }
            if y < height - 1 {
                if problem.2[y][x] {
                    board.push(Item {
                        y: y * 2 + 2,
                        x: x * 2 + 1,
                        color: "black",
                        kind: ItemKind::BoldWall,
                    });
                }
                if let Some(true) = v_edges[y][x] {
                    board.push(Item {
                        y: y * 2 + 2,
                        x: x * 2 + 1,
                        color: "green",
                        kind: ItemKind::Line,
                    });
                }
            }
        }
    }

    Ok(board)
}
