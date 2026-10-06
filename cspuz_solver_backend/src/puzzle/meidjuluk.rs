use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::{is_unique, Uniqueness};
use cspuz_rs_puzzles::puzzles::meidjuluk;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = meidjuluk::deserialize_problem(url).ok_or("invalid url")?;
    let (h_edges, v_edges, truncated) =
        meidjuluk::solve_meidjuluk(&problem).ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.0[0].len();
    let mut board = Board::new(
        BoardKind::Grid,
        height,
        width,
        if truncated {
            Uniqueness::NonUnique
        } else {
            is_unique(&(&h_edges, &v_edges))
        },
    );

    let is_shaded = |y: usize, x: usize| problem.0[y][x] == Some(0);
    for y in 0..height {
        for x in 0..width {
            match problem.0[y][x] {
                Some(0) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Fill));
                }
                Some(n) if n > 0 => {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                }
                _ => {}
            }
            // Borders adjacent to shaded cells are not dotted lines: the
            // solver never reports them, and pzprjs draws them solid black.
            // Drawn lines are shown as bold walls; edges that must stay
            // open are marked with a cross (same as fillomino / araf).
            if x < width - 1 && !is_shaded(y, x) && !is_shaded(y, x + 1) {
                match h_edges[y][x] {
                    Some(true) => board.push(Item {
                        y: y * 2 + 1,
                        x: x * 2 + 2,
                        color: "green",
                        kind: ItemKind::BoldWall,
                    }),
                    Some(false) => board.push(Item {
                        y: y * 2 + 1,
                        x: x * 2 + 2,
                        color: "green",
                        kind: ItemKind::Cross,
                    }),
                    None => {}
                }
            }
            if y < height - 1 && !is_shaded(y, x) && !is_shaded(y + 1, x) {
                match v_edges[y][x] {
                    Some(true) => board.push(Item {
                        y: y * 2 + 2,
                        x: x * 2 + 1,
                        color: "green",
                        kind: ItemKind::BoldWall,
                    }),
                    Some(false) => board.push(Item {
                        y: y * 2 + 2,
                        x: x * 2 + 1,
                        color: "green",
                        kind: ItemKind::Cross,
                    }),
                    None => {}
                }
            }
        }
    }

    Ok(board)
}
