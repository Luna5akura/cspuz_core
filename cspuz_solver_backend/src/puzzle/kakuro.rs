use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::kakuro;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = kakuro::deserialize_problem(url).ok_or("invalid url")?;
    let answer: Vec<Vec<Option<i32>>> = kakuro::solve_kakuro(&problem).ok_or("no answer")?;

    // `deserialize_problem` normally produces a rectangular matrix with a
    // one-cell synthetic frame.  Keep the boundary explicit here: this
    // function is also the WASM API boundary, so a malformed/custom decoder
    // must result in a regular error instead of an indexing panic below.
    let problem_height = problem.len();
    let problem_width = problem.first().map(|row| row.len()).unwrap_or(0);
    if problem_height < 2
        || problem_width < 2
        || problem.iter().any(|row| row.len() != problem_width)
        || answer.len() != problem_height
        || answer.iter().any(|row| row.len() != problem_width)
    {
        return Err("invalid answer");
    }

    // The Kakuro serializer keeps one synthetic row and column in the
    // solver matrix for the clues that live outside the playable grid.  A
    // Board description, however, is consumed by pzpr as a zero-based
    // playable grid.  Do not expose that synthetic frame: doing so shifts
    // every answer one cell down and to the right in the UI.
    let height = problem_height - 1;
    let width = problem_width - 1;
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&answer));

    for y in 1..=height {
        for x in 1..=width {
            let (by, bx) = (y - 1, x - 1);
            if problem[y][x].is_none() {
                if let Some(n) = answer[y][x] {
                    board.push(Item::cell(by, bx, "green", ItemKind::Num(n)));
                }
            }
        }
    }

    Ok(board)
}
