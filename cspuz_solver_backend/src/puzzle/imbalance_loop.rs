use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs::graph;
use cspuz_rs_puzzles::puzzles::imbalance_loop;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = imbalance_loop::deserialize_problem(url).ok_or("invalid url")?;
    let is_line = imbalance_loop::solve_imbalance_loop(&problem).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&is_line));

    let mut skip_line = vec![];
    for y in 0..height {
        let mut row = vec![];
        for x in 0..width {
            row.push(problem[y][x].black);
        }
        skip_line.push(row);
    }
    for y in 0..height {
        for x in 0..width {
            let cell = problem[y][x];
            if cell.black {
                board.push(Item::cell(y, x, "black", ItemKind::Fill));
            }
            if let Some(clue) = cell.clue {
                let arrow = match clue.dir {
                    Some(imbalance_loop::ImbDir::Up) => Some(ItemKind::SideArrowUp),
                    Some(imbalance_loop::ImbDir::Down) => Some(ItemKind::SideArrowDown),
                    Some(imbalance_loop::ImbDir::Left) => Some(ItemKind::SideArrowLeft),
                    Some(imbalance_loop::ImbDir::Right) => Some(ItemKind::SideArrowRight),
                    None => None,
                };
                if let Some(arrow) = arrow {
                    board.push(Item::cell(y, x, "black", arrow));
                }
                board.push(Item::cell(
                    y,
                    x,
                    "black",
                    match clue.num {
                        Some(n) => ItemKind::Num(n),
                        None => ItemKind::Text("?"),
                    },
                ));
            }
        }
    }

    board.add_lines_irrefutable_facts(&is_line, "green", Some(&skip_line));

    Ok(board)
}
