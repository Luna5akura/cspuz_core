use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::yinyang_mines::{self, YinYangMinesClue};

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = yinyang_mines::deserialize_problem(url).ok_or("invalid url")?;
    let ans = yinyang_mines::solve_yinyang_mines(&problem).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&ans));

    for y in 0..height {
        for x in 0..width {
            match problem[y][x] {
                YinYangMinesClue::None => (),
                YinYangMinesClue::White => board.push(Item::cell(y, x, "black", ItemKind::Circle)),
                YinYangMinesClue::Black => {
                    board.push(Item::cell(y, x, "black", ItemKind::FilledCircle))
                }
                YinYangMinesClue::Number(n) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Circle));
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                }
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            if let Some(b) = ans[y][x] {
                board.push(Item::cell(
                    y,
                    x,
                    "green",
                    if b {
                        ItemKind::FilledCircle
                    } else {
                        ItemKind::Circle
                    },
                ));
            }
        }
    }

    Ok(board)
}
