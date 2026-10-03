use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::even_loop_kakuro;

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let problem = even_loop_kakuro::deserialize_problem(url).ok_or("invalid url")?;
    let (numbers, lines) = even_loop_kakuro::solve_even_loop_kakuro(&problem).ok_or("no answer")?;

    // deserialize_problem は盤面の上と左にヒント用の合成フレームを1マス分
    // 付ける。Board description は pzpr 側のプレイ可能領域を0基準で扱うため、
    // フレームを除かないと解答全体が右下に1マスずれて表示される。
    let problem_height = numbers.len();
    let problem_width = numbers[0].len();
    if problem_height < 2
        || problem_width < 2
        || numbers.iter().any(|row| row.len() != problem_width)
    {
        return Err("invalid answer");
    }
    let height = problem_height - 1;
    let width = problem_width - 1;
    let mut board = Board::new(
        BoardKind::Grid,
        height,
        width,
        is_unique(&(&numbers, &lines)),
    );

    for y in 1..=height {
        for x in 1..=width {
            if problem[y][x].is_none() {
                if let Some(n) = numbers[y][x] {
                    board.push(Item::cell(y - 1, x - 1, "green", ItemKind::Num(n)));
                }
            }
        }
    }

    // ループ線: プレイ可能領域の内部の辺のみを出力する
    for y in 0..height {
        for x in 0..width {
            if y < height - 1 {
                if let Some(b) = lines.horizontal[y + 2][x + 1] {
                    board.push(Item {
                        y: y * 2 + 2,
                        x: x * 2 + 1,
                        color: "green",
                        kind: if b {
                            ItemKind::Line
                        } else {
                            ItemKind::Cross
                        },
                    });
                }
            }
            if x < width - 1 {
                if let Some(b) = lines.vertical[y + 1][x + 2] {
                    board.push(Item {
                        y: y * 2 + 1,
                        x: x * 2 + 2,
                        color: "green",
                        kind: if b {
                            ItemKind::Line
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
