use crate::board::{Board, BoardKind, Item, ItemKind};
use cspuz_rs_puzzles::puzzles::battleships;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = battleships::deserialize_problem(url).ok_or("invalid url")?;
    let ans = battleships::solve_battleships(&problem).ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.0[0].len();
    let uniqueness = if ans.is_unique {
        crate::uniqueness::Uniqueness::Unique
    } else {
        crate::uniqueness::Uniqueness::NonUnique
    };
    let mut board = Board::new(BoardKind::Grid, height, width, uniqueness);

    for y in 0..height {
        for x in 0..width {
            if ans.covered[y][x] == Some(true) {
                // プレイヤーが艦を塗るときと同じ色 (statuepark の shadecolor)
                board.push(Item::cell(y, x, "rgb(80, 80, 80)", ItemKind::Fill));
            }
        }
    }

    Ok(board)
}
