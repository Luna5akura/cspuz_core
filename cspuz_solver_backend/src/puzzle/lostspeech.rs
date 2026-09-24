use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::lostspeech;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = lostspeech::deserialize_problem(url).ok_or("invalid url")?;
    let ans = lostspeech::solve_lostspeech_display(&problem.0, &problem.1, &problem.2)
        .ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.0[0].len();
    let uniqueness = if ans.is_unique {
        crate::uniqueness::Uniqueness::Unique
    } else {
        crate::uniqueness::Uniqueness::NonUnique
    };
    let mut board = Board::new(BoardKind::Grid, height, width, uniqueness);

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
            let is_blue = ans.blue_cells[y][x] == Some(true);
            let is_red = ans.red_cells[y][x] == Some(true);
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

    // 確定した形状インスタンスごとの境界線を描く
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

    // 表示用の形状分解 (1つの解) から全形状のグループを決める。
    // 境界線は「確定事実のマス」に接しているものだけを描く。
    let mut blue_group = vec![vec![None; width]; height];
    let mut red_group = vec![vec![None; width]; height];
    for (i, cells) in blue_placements.iter().enumerate() {
        if ans.blue_placements[i] == Some(true) {
            for &(y, x) in cells {
                blue_group[y][x] = Some(i);
            }
        }
    }
    for (i, cells) in red_placements.iter().enumerate() {
        if ans.red_placements[i] == Some(true) {
            for &(y, x) in cells {
                red_group[y][x] = Some(i);
            }
        }
    }

    // 確定事実のマスから、上下左右すべての境界を出力する。
    // 隣接マスが「同じ形状の一部で、かつ確定事実」の場合のみ線を引かない。
    let mut emitted = std::collections::BTreeSet::new();
    for y in 0..height {
        for x in 0..width {
            if problem.1[y][x] {
                continue;
            }
            let is_fact_blue = ans.blue_cells[y][x] == Some(true);
            let is_fact_red = ans.red_cells[y][x] == Some(true);
            if !is_fact_blue && !is_fact_red {
                continue;
            }
            let bg = blue_group[y][x];
            let rg = red_group[y][x];
            let color = if is_fact_blue && is_fact_red {
                GRAYED_BOTH_LINE
            } else if is_fact_blue {
                GRAYED_BLUE_LINE
            } else {
                GRAYED_RED_LINE
            };

            let neighbor_is_same_shape = |y2: usize, x2: usize| -> bool {
                if problem.1[y2][x2] {
                    return false;
                }
                // 両方の色について、確定事実かつグループが一致する
                // 場合のみ「同じ形状の一部」とみなす
                let mut same = true;
                if is_fact_blue {
                    same = same
                        && ans.blue_cells[y2][x2] == Some(true)
                        && blue_group[y2][x2].is_some()
                        && blue_group[y2][x2] == bg;
                }
                if is_fact_red {
                    same = same
                        && ans.red_cells[y2][x2] == Some(true)
                        && red_group[y2][x2].is_some()
                        && red_group[y2][x2] == rg;
                }
                same
            };

            let draw_right = if x + 1 < width {
                !neighbor_is_same_shape(y, x + 1)
            } else {
                true
            };
            let draw_bottom = if y + 1 < height {
                !neighbor_is_same_shape(y + 1, x)
            } else {
                true
            };
            let draw_top = if y > 0 {
                !neighbor_is_same_shape(y - 1, x)
            } else {
                true
            };
            let draw_left = if x > 0 {
                !neighbor_is_same_shape(y, x - 1)
            } else {
                true
            };

            let borders = [
                // 右 (垂直線: y*2+1, x*2+2)
                (draw_right, y * 2 + 1, x * 2 + 2),
                // 下 (水平線: y*2+2, x*2+1)
                (draw_bottom, y * 2 + 2, x * 2 + 1),
                // 上 (水平線: y*2, x*2+1)
                (draw_top, y * 2, x * 2 + 1),
                // 左 (垂直線: y*2+1, x*2)
                (draw_left, y * 2 + 1, x * 2),
            ];
            for &(cond, by, bx) in &borders {
                if cond && emitted.insert((by, bx)) {
                    board.push(Item {
                        y: by,
                        x: bx,
                        color,
                        kind: ItemKind::Wall,
                    });
                }
            }
        }
    }

    Ok(board)
}
