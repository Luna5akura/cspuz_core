use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::echo;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = echo::deserialize_problem(url).ok_or("invalid url")?;
    let ans = echo::solve_echo(&problem).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&ans));
    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = &problem[y][x] {
                for &e in clue {
                    board.push(Item::cell(
                        y,
                        x,
                        "black",
                        if e < 0 {
                            ItemKind::Text("?")
                        } else {
                            ItemKind::Num(e)
                        },
                    ));
                }
            } else if let Some(a) = ans[y][x] {
                board.push(Item::cell(
                    y,
                    x,
                    "green",
                    if a { ItemKind::Fill } else { ItemKind::Dot },
                ));
            }
        }
    }

    Ok(board)
}
