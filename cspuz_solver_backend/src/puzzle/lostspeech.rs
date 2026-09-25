use crate::board::{Board, BoardKind, Item, ItemKind};
use cspuz_rs_puzzles::puzzles::lostspeech;

// 2つの同一盤面を左右に並べて表示するためのxオフセット (仮想座標)
fn twin_offset(width: usize) -> usize {
    2 * width + 2
}

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = lostspeech::deserialize_problem(url).ok_or("invalid url")?;
    let ans = lostspeech::solve_lostspeech_facts(&problem.0, &problem.1, &problem.2)
        .ok_or("no answer")?;

    let height = problem.0.len();
    let width = problem.0[0].len();
    let off = twin_offset(width);
    // 2つの盤面 + 隙間1マス分
    let board_width = 2 * width + 1;
    let uniqueness = if ans.is_unique {
        crate::uniqueness::Uniqueness::Unique
    } else {
        crate::uniqueness::Uniqueness::NonUnique
    };
    let mut board = Board::new(BoardKind::Grid, height, board_width, uniqueness);

    // プレイヤーが置いた図形と同じようにセル全体を塗る。
    // プレイヤーの図形より少し灰色がかった半透明の色を使う。
    const GRAYED_BLUE: &str = "rgba(96, 118, 196, 0.35)";
    const GRAYED_RED: &str = "rgba(206, 108, 108, 0.35)";
    const GRAYED_BOTH: &str = "rgba(146, 112, 176, 0.35)";

    for y in 0..height {
        for x in 0..width {
            if problem.1[y][x] {
                board.push(Item::cell(y, x, "black", ItemKind::Fill));
                board.push(Item::cell(y, x + off, "black", ItemKind::Fill));
                continue;
            }
            for (bx, is_blue, is_red) in [
                (x, ans.blue1_cells[y][x] == Some(true), ans.red1_cells[y][x] == Some(true)),
                (
                    x + off,
                    ans.blue2_cells[y][x] == Some(true),
                    ans.red2_cells[y][x] == Some(true),
                ),
            ] {
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
                    board.push(Item::cell(y, bx, color, ItemKind::Fill));
                }
            }
        }
    }

    // 各ピース (青1, 赤1, 青2, 赤2) の配置一覧
    let placements: Vec<Vec<Vec<(usize, usize)>>> = problem
        .2
        .iter()
        .map(|p| lostspeech::gen_placements(&[p.clone()], &problem.0, &problem.1))
        .collect();
    let placements = |i: usize| -> &[Vec<(usize, usize)>] {
        placements.get(i).map(|v| v.as_slice()).unwrap_or(&[])
    };

    // 盤面ごとに、セルを覆う「確定形状」(全解で必ず配置される形状)を記録する。
    // 確定形状同士は重なれないため、1セルにつき高々1つ。
    let mut blue1_pc = vec![vec![None::<usize>; width]; height];
    let mut red1_pc = vec![vec![None::<usize>; width]; height];
    let mut blue2_pc = vec![vec![None::<usize>; width]; height];
    let mut red2_pc = vec![vec![None::<usize>; width]; height];
    for (i, cells) in placements(0).iter().enumerate() {
        if ans.blue1_placements.get(i) == Some(&Some(true)) {
            for &(y, x) in cells {
                blue1_pc[y][x] = Some(i);
            }
        }
    }
    for (i, cells) in placements(1).iter().enumerate() {
        if ans.red1_placements.get(i) == Some(&Some(true)) {
            for &(y, x) in cells {
                red1_pc[y][x] = Some(i);
            }
        }
    }
    for (i, cells) in placements(2).iter().enumerate() {
        if ans.blue2_placements.get(i) == Some(&Some(true)) {
            for &(y, x) in cells {
                blue2_pc[y][x] = Some(i);
            }
        }
    }
    for (i, cells) in placements(3).iter().enumerate() {
        if ans.red2_placements.get(i) == Some(&Some(true)) {
            for &(y, x) in cells {
                red2_pc[y][x] = Some(i);
            }
        }
    }

    // 形状の境界線は「どの解でも必ず境界になっている」場合のみ描く。
    // 隣接する2マスについて青/赤それぞれ:
    //   - 同じ確定形状に含まれる → 必ず同じ形状 (線なし)
    //   - どちらかが確定形状に含まれ、相手がその形状に含まれない
    //     → 必ず別の形状 (線あり)
    //   - どちらも確定形状に含まれない → 分かれ方は解による (線なし)
    let mut emitted = std::collections::BTreeSet::new();
    for y in 0..height {
        for x in 0..width {
            if problem.1[y][x] {
                continue;
            }
            let fact_blue1 = ans.blue1_cells[y][x] == Some(true);
            let fact_red1 = ans.red1_cells[y][x] == Some(true);
            let fact_blue2 = ans.blue2_cells[y][x] == Some(true);
            let fact_red2 = ans.red2_cells[y][x] == Some(true);

            // 盤面1
            draw_board_edges(
                &mut board,
                &mut emitted,
                &problem.1,
                y,
                x,
                0,
                fact_blue1,
                fact_red1,
                &blue1_pc,
                &red1_pc,
            );
            // 盤面2
            draw_board_edges(
                &mut board,
                &mut emitted,
                &problem.1,
                y,
                x,
                off,
                fact_blue2,
                fact_red2,
                &blue2_pc,
                &red2_pc,
            );
        }
    }

    Ok(board)
}

#[allow(clippy::too_many_arguments)]
fn draw_board_edges(
    board: &mut Board,
    emitted: &mut std::collections::BTreeSet<(usize, usize)>,
    invalid: &[Vec<bool>],
    y: usize,
    x: usize,
    xoff: usize,
    fact_blue: bool,
    fact_red: bool,
    blue_pc: &[Vec<Option<usize>>],
    red_pc: &[Vec<Option<usize>>],
) {
    if !fact_blue && !fact_red {
        return;
    }
    let height = invalid.len();
    let width = invalid[0].len();

    // 盤面端・灰色マスとの境界は必ず形状の境界になる
    let fact_color: &'static str = if fact_blue && fact_red {
        GRAYED_BOTH_LINE
    } else if fact_blue {
        GRAYED_BLUE_LINE
    } else {
        GRAYED_RED_LINE
    };

    // 盤面内の隣接マスとの境界が確定している場合の線の色を返す
    let edge_diff_color = |y2: usize, x2: usize| -> Option<&'static str> {
        let blue_diff = (blue_pc[y][x].is_some() || blue_pc[y2][x2].is_some())
            && blue_pc[y][x] != blue_pc[y2][x2];
        let red_diff = (red_pc[y][x].is_some() || red_pc[y2][x2].is_some())
            && red_pc[y][x] != red_pc[y2][x2];
        if blue_diff && red_diff {
            Some(GRAYED_BOTH_LINE)
        } else if blue_diff {
            Some(GRAYED_BLUE_LINE)
        } else if red_diff {
            Some(GRAYED_RED_LINE)
        } else {
            None
        }
    };

    // 右 (垂直線: y*2+1, x*2+2)
    let right = if x + 1 < width {
        if invalid[y][x + 1] {
            Some((y * 2 + 1, x * 2 + 2 + xoff * 2, fact_color))
        } else {
            edge_diff_color(y, x + 1).map(|c| (y * 2 + 1, x * 2 + 2 + xoff * 2, c))
        }
    } else {
        Some((y * 2 + 1, x * 2 + 2 + xoff * 2, fact_color))
    };
    // 下 (水平線: y*2+2, x*2+1)
    let bottom = if y + 1 < height {
        if invalid[y + 1][x] {
            Some((y * 2 + 2, x * 2 + 1 + xoff * 2, fact_color))
        } else {
            edge_diff_color(y + 1, x).map(|c| (y * 2 + 2, x * 2 + 1 + xoff * 2, c))
        }
    } else {
        Some((y * 2 + 2, x * 2 + 1 + xoff * 2, fact_color))
    };
    // 上 (水平線: y*2, x*2+1)
    let top = if y > 0 {
        if invalid[y - 1][x] {
            Some((y * 2, x * 2 + 1 + xoff * 2, fact_color))
        } else {
            edge_diff_color(y - 1, x).map(|c| (y * 2, x * 2 + 1 + xoff * 2, c))
        }
    } else {
        Some((y * 2, x * 2 + 1 + xoff * 2, fact_color))
    };
    // 左 (垂直線: y*2+1, x*2)
    let left = if x > 0 {
        if invalid[y][x - 1] {
            Some((y * 2 + 1, x * 2 + xoff * 2, fact_color))
        } else {
            edge_diff_color(y, x - 1).map(|c| (y * 2 + 1, x * 2 + xoff * 2, c))
        }
    } else {
        Some((y * 2 + 1, x * 2 + xoff * 2, fact_color))
    };

    for edge in [right, bottom, top, left] {
        if let Some((by, bx, color)) = edge {
            if emitted.insert((by, bx)) {
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

// 定数参照用 (draw_board_edges から使用)
const GRAYED_BLUE_LINE: &str = "rgba(96, 118, 196, 0.9)";
const GRAYED_RED_LINE: &str = "rgba(206, 108, 108, 0.9)";
const GRAYED_BOTH_LINE: &str = "rgba(146, 112, 176, 0.9)";
