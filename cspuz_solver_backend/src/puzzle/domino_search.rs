use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::domino_search;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = domino_search::deserialize_problem(url).ok_or("invalid url")?;
    let borders = domino_search::solve_domino_search(&problem).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(BoardKind::OuterGrid, height, width, is_unique(&borders));

    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = problem[y][x] {
                if clue >= 0 {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(clue)));
                }
            } else {
                board.push(Item::cell(y, x, "black", ItemKind::Block));
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            if y < height - 1 {
                if is_number_edge(&problem, (y, x), (y + 1, x)) {
                    if borders.horizontal[y][x] == Some(true) {
                        board.push(Item {
                            y: y * 2 + 2,
                            x: x * 2 + 1,
                            color: "green",
                            kind: ItemKind::BoldWall,
                        });
                    }
                }
            }
            if x < width - 1 {
                if is_number_edge(&problem, (y, x), (y, x + 1)) {
                    if borders.vertical[y][x] == Some(true) {
                        board.push(Item {
                            y: y * 2 + 1,
                            x: x * 2 + 2,
                            color: "green",
                            kind: ItemKind::BoldWall,
                        });
                    }
                }
            }
        }
    }

    Ok(board)
}

fn is_number_edge(
    problem: &[Vec<Option<i32>>],
    c1: (usize, usize),
    c2: (usize, usize),
) -> bool {
    problem[c1.0][c1.1].is_some() && problem[c2.0][c2.1].is_some()
}
