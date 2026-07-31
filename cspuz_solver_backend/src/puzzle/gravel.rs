use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::Uniqueness;
use cspuz_rs_puzzles::puzzles::gravel::{self, Circle};

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = gravel::deserialize_problem(url).ok_or("invalid url")?;
    let ans = gravel::solve_gravel(&problem).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(
        BoardKind::OuterGrid,
        height,
        width,
        if ans.is_unique {
            Uniqueness::Unique
        } else {
            Uniqueness::NonUnique
        },
    );

    for y in 0..height {
        for x in 0..width {
            let clue = problem[y][x];
            if clue.empty {
                board.push(Item::cell(y, x, "black", ItemKind::Cross));
                continue;
            }
            match clue.circle {
                Some(Circle::White) => board.push(Item::cell(y, x, "black", ItemKind::Circle)),
                Some(Circle::Black) => {
                    board.push(Item::cell(y, x, "black", ItemKind::FilledCircle))
                }
                None => {}
            }
            if let Some(n) = clue.number {
                if n == -2 {
                    board.push(Item::cell(y, x, "black", ItemKind::Text("?")));
                } else if n >= 0 {
                    board.push(Item::cell(y, x, "black", ItemKind::Num(n)));
                }
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            if !problem[y][x].empty
                && problem[y][x].circle.is_none()
                && problem[y][x].number.is_none()
            {
                if let Some(is_black) = ans.is_black[y][x] {
                    board.push(Item::cell(
                        y,
                        x,
                        "green",
                        if is_black {
                            ItemKind::Block
                        } else {
                            ItemKind::Dot
                        },
                    ));
                }
            }
            if y + 1 < height {
                if let Some(b) = border_state_to_show(&problem, &ans, (y, x), (y + 1, x)) {
                    board.push(Item {
                        y: y * 2 + 2,
                        x: x * 2 + 1,
                        color: "green",
                        kind: if b {
                            ItemKind::BoldWall
                        } else {
                            ItemKind::Cross
                        },
                    });
                }
            }
            if x + 1 < width {
                if let Some(b) = border_state_to_show(&problem, &ans, (y, x), (y, x + 1)) {
                    board.push(Item {
                        y: y * 2 + 1,
                        x: x * 2 + 2,
                        color: "green",
                        kind: if b {
                            ItemKind::BoldWall
                        } else {
                            ItemKind::Cross
                        },
                    });
                }
            }
        }
    }

    Ok(board)
}

fn border_state_to_show(
    problem: &gravel::Problem,
    ans: &gravel::GravelAnswer,
    c1: (usize, usize),
    c2: (usize, usize),
) -> Option<bool> {
    let (y1, x1) = c1;
    let (y2, x2) = c2;
    if problem[y1][x1].empty || problem[y2][x2].empty {
        return None;
    }
    let border = if y1 == y2 {
        ans.borders.vertical[y1][x1.min(x2)]
    } else {
        ans.borders.horizontal[y1.min(y2)][x1]
    };
    match border {
        Some(true) => Some(true),
        Some(false)
            if ans.is_black[y1][x1] == Some(false)
                && ans.is_black[y2][x2] == Some(false) =>
        {
            Some(false)
        }
        _ => None,
    }
}
