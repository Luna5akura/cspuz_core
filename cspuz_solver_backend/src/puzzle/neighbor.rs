use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::neighbor;

/// Solve a Neighbors PuzzLink URL and turn the result into the generic board
/// description consumed by the pzprjs solver UI.
pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = neighbor::deserialize_problem(url).ok_or("invalid url")?;
    let ans = neighbor::solve_neighbors(&problem).ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.0[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&ans));

    for y in 0..height {
        for x in 0..width {
            // Outlined cells are part of the puzzle definition, so draw them
            // as black square marks independently of the number layer.
            if problem.1[y][x] {
                board.push(Item::cell(y, x, "black", ItemKind::Square));
            }

            if let Some(given) = problem.0[y][x] {
                board.push(Item::cell(y, x, "black", ItemKind::Num(given)));
            } else if let Some(value) = ans[y][x] {
                // Green entries represent solver-derived values.  The pzprjs
                // overlay deliberately ignores these when a user answer is
                // already present in the cell.
                board.push(Item::cell(y, x, "green", ItemKind::Num(value)));
            }
        }
    }

    Ok(board)
}
