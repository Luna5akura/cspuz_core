use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::anglers;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = anglers::deserialize_problem(url).ok_or("invalid url")?;
    let (h_edges, v_edges, truncated) = anglers::solve_anglers(&problem).ok_or("no answer")?;

    let (cells, excs) = &problem;
    let height = cells.len();
    let width = cells[0].len();
    let mut board = Board::new(
        BoardKind::Empty,
        height * 2 + 1,
        width * 2 + 1,
        if truncated {
            crate::uniqueness::Uniqueness::NonUnique
        } else {
            is_unique(&h_edges)
        },
    );

    // Lattice edges as line items: h_edge(y, x) connects grid points
    // (y, x) and (y, x+1); v_edge(y, x) connects (y, x) and (y+1, x).
    for y in 0..=height {
        for x in 0..width {
            if let Some(true) = h_edges[y][x] {
                board.push(Item {
                    y: y * 2,
                    x: x * 2 + 1,
                    color: "green",
                    kind: ItemKind::Line,
                });
            }
        }
    }
    for y in 0..height {
        for x in 0..=width {
            if let Some(true) = v_edges[y][x] {
                board.push(Item {
                    y: y * 2 + 1,
                    x: x * 2,
                    color: "green",
                    kind: ItemKind::Line,
                });
            }
        }
    }

    // Cell clues: fish (0), numbers (>0), ? (-2), obstacles (-3).
    for y in 0..height {
        for x in 0..width {
            match cells[y][x] {
                Some(-3) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Fill));
                }
                Some(0) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Dot));
                }
                Some(n) if n > 0 => {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                }
                Some(-2) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Text("?")));
                }
                _ => {}
            }
        }
    }
    // Grid-point clues.
    for y in 0..=height {
        for x in 0..=width {
            match excs[y][x] {
                Some(n) if n > 0 => {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                }
                Some(-2) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Text("?")));
                }
                _ => {}
            }
        }
    }

    Ok(board)
}
