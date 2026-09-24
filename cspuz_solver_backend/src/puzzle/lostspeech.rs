use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::lostspeech;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = lostspeech::deserialize_problem(url).ok_or("invalid url")?;
    let ans =
        lostspeech::solve_lostspeech(&problem.0, &problem.1, &problem.2).ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.0[0].len();
    let mut board = Board::new(BoardKind::Grid, height, width, is_unique(&(&ans.0, &ans.1)));

    // プレイヤーが置いた図形と同じようにセル全体を塗る。
    // プレイヤーの図形より少し灰色がかった半透明の色を使う。
    const GRAYED_BLUE: &str = "rgba(96, 118, 196, 0.35)";
    const GRAYED_RED: &str = "rgba(206, 108, 108, 0.35)";
    const GRAYED_BOTH: &str = "rgba(146, 112, 176, 0.35)";
    const GRAYED_BLUE_LINE: &str = "rgba(96, 118, 196, 0.9)";
    const GRAYED_RED_LINE: &str = "rgba(206, 108, 108, 0.9)";
    const GRAYED_BOTH_LINE: &str = "rgba(146, 112, 176, 0.9)";

    for y in 0..height {
        for x in 0..width {
            if problem.1[y][x] {
                board.push(Item::cell(y, x, "black", ItemKind::Fill));
                continue;
            }
            let is_blue = ans.0[y][x] == Some(true);
            let is_red = ans.1[y][x] == Some(true);
            let color = if is_blue && is_red {
                Some(GRAYED_BOTH)
            } else if is_blue {
                Some(GRAYED_BLUE)
            } else if is_red {
                Some(GRAYED_RED)
            } else {
                None
            };
            if let Some(color) = color {
                board.push(Item::cell(y, x, color, ItemKind::Fill));
            }
        }
    }

    // 確定した形状インスタンスごとの境界線を描く。
    // 隣接するセルが異なる形状(または形状なし)に属する場合に境界線を引く。
    let blue_placements = lostspeech::gen_placements(
        &problem
            .2
            .iter()
            .enumerate()
            .filter_map(|(i, p)| if i % 2 == 0 { Some(p.clone()) } else { None })
            .collect::<Vec<_>>(),
        &problem.0,
        &problem.1,
    );
    let red_placements = lostspeech::gen_placements(
        &problem
            .2
            .iter()
            .enumerate()
            .filter_map(|(i, p)| if i % 2 == 1 { Some(p.clone()) } else { None })
            .collect::<Vec<_>>(),
        &problem.0,
        &problem.1,
    );

    let mut blue_group = vec![vec![None; width]; height];
    let mut red_group = vec![vec![None; width]; height];
    for (i, cells) in blue_placements.iter().enumerate() {
        if ans.2[i] == Some(true) {
            for &(y, x) in cells {
                blue_group[y][x] = Some(i);
            }
        }
    }
    for (i, cells) in red_placements.iter().enumerate() {
        if ans.3[i] == Some(true) {
            for &(y, x) in cells {
                red_group[y][x] = Some(i);
            }
        }
    }

    let is_group_border = |y1: usize, x1: usize, y2: usize, x2: usize| {
        if problem.1[y1][x1] || problem.1[y2][x2] {
            return false;
        }
        let blue_diff = blue_group[y1][x1] != blue_group[y2][x2];
        let red_diff = red_group[y1][x1] != red_group[y2][x2];
        (blue_diff || red_diff)
            && (blue_group[y1][x1].is_some()
                || blue_group[y2][x2].is_some()
                || red_group[y1][x1].is_some()
                || red_group[y2][x2].is_some())
    };

    for y in 0..height {
        for x in 0..width {
            if problem.1[y][x] {
                continue;
            }
            let bg = blue_group[y][x];
            let rg = red_group[y][x];
            if bg.is_none() && rg.is_none() {
                continue;
            }

            // 右のセルとの境界
            if x + 1 < width && is_group_border(y, x, y, x + 1) {
                let blue_diff = blue_group[y][x] != blue_group[y][x + 1];
                let red_diff = red_group[y][x] != red_group[y][x + 1];
                let color = if blue_diff && red_diff {
                    GRAYED_BOTH_LINE
                } else if blue_diff {
                    GRAYED_BLUE_LINE
                } else {
                    GRAYED_RED_LINE
                };
                board.push(Item {
                    y: y * 2 + 1,
                    x: x * 2 + 2,
                    color,
                    kind: ItemKind::Wall,
                });
            }
            // 下のセルとの境界
            if y + 1 < height && is_group_border(y, x, y + 1, x) {
                let blue_diff = blue_group[y][x] != blue_group[y + 1][x];
                let red_diff = red_group[y][x] != red_group[y + 1][x];
                let color = if blue_diff && red_diff {
                    GRAYED_BOTH_LINE
                } else if blue_diff {
                    GRAYED_BLUE_LINE
                } else {
                    GRAYED_RED_LINE
                };
                board.push(Item {
                    y: y * 2 + 2,
                    x: x * 2 + 1,
                    color,
                    kind: ItemKind::Wall,
                });
            }
            // 盤面の外枠との境界
            let color = if bg.is_some() && rg.is_some() {
                GRAYED_BOTH_LINE
            } else if bg.is_some() {
                GRAYED_BLUE_LINE
            } else {
                GRAYED_RED_LINE
            };
            if y == 0 {
                board.push(Item {
                    y: 0,
                    x: x * 2 + 1,
                    color,
                    kind: ItemKind::Wall,
                });
            }
            if x == 0 {
                board.push(Item {
                    y: y * 2 + 1,
                    x: 0,
                    color,
                    kind: ItemKind::Wall,
                });
            }
            if y + 1 == height {
                board.push(Item {
                    y: y * 2 + 2,
                    x: x * 2 + 1,
                    color,
                    kind: ItemKind::Wall,
                });
            }
            if x + 1 == width {
                board.push(Item {
                    y: y * 2 + 1,
                    x: x * 2 + 2,
                    color,
                    kind: ItemKind::Wall,
                });
            }
        }
    }

    Ok(board)
}
