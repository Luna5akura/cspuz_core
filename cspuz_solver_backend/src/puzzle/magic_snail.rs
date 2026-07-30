use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::magic_snail;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = magic_snail::deserialize_problem(url).ok_or("invalid url")?;
    let ans = magic_snail::solve_magic_snail(&problem).ok_or("no answer")?;

    let height = problem.cell_clues.len();
    let width = problem.cell_clues[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&ans));

    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = problem.cell_clues[y][x] {
                if clue > 0 {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(clue)));
                } else if clue == -2 {
                    board.push(Item::cell(y, x, "black", ItemKind::Text("?")));
                }
            } else if let Some(n) = ans[y][x] {
                board.push(Item::cell(
                    y,
                    x,
                    "green",
                    if n > 0 {
                        ItemKind::Num(n)
                    } else {
                        ItemKind::Cross
                    },
                ));
            }
        }
    }

    Ok(board)
}
