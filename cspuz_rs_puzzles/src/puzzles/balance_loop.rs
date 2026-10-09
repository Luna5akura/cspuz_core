use cspuz_rs::graph;
use cspuz_rs::serializer::strip_prefix;
use cspuz_rs::solver::{count_true, BoolExpr, IntExpr, Solver, FALSE, TRUE};

pub const DIRECTIONS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)]; // Up, Down, Left, Right

/// 手がかり: white = 白丸 (true) / 黒丸 (false)。num = None は ? を表す。
///
/// Balance Loop のルール (pzprjs balance.js の判定と一致):
/// - 一つのループを引く (分岐・交差なし。空きマスはループが通らなくてもよい)。
/// - 白丸のマス: 通る 2 本の直線セグメントの長さが等しい。
/// - 黒丸のマス: 通る 2 本の直線セグメントの長さが異なる。
/// - 数字 n: 通る 2 本の直線セグメントの長さの和が n (数字マスは必ずループ上)。
/// - ? マスも必ずループ上 (UI の checkNoLineCircle は isNum() で ? も対象)。
/// - 'f' フラグ (loop_full): ループが全マスを通る (空きマスも含む)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Clue {
    pub white: bool,
    pub num: Option<i32>,
}

pub type Problem = Vec<Vec<Option<Clue>>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BalanceLoopOptions {
    /// 'f' フラグ: ループが全マスを通る
    pub loop_full: bool,
}

pub fn solve_balance_loop(problem: &Problem) -> Option<graph::BoolGridEdgesIrrefutableFacts> {
    solve_balance_loop_with_options(problem, BalanceLoopOptions { loop_full: false })
}

pub fn solve_balance_loop_with_options(
    problem: &Problem,
    opts: BalanceLoopOptions,
) -> Option<graph::BoolGridEdgesIrrefutableFacts> {
    let h = problem.len();
    let w = problem[0].len();

    let mut solver = Solver::new();
    // 盤面の内側の辺のみ (outer frame は含めない)
    let edges = graph::BoolGridEdges::new(&mut solver, (h - 1, w - 1));
    solver.add_answer_key_bool(&edges.horizontal);
    solver.add_answer_key_bool(&edges.vertical);
    graph::single_cycle_grid_edges(&mut solver, &edges);

    // セル (y, x) の d 方向の辺 (盤外なら FALSE)
    let border = |y: i32, x: i32, d: usize| -> BoolExpr {
        let (y, x) = (y as usize, x as usize);
        match d {
            0 => edges.vertical.at_offset((y, x), (-1, 0), FALSE),   // Up
            1 => edges.vertical.at_offset((y, x), (0, 0), FALSE),    // Down
            2 => edges.horizontal.at_offset((y, x), (0, -1), FALSE), // Left
            3 => edges.horizontal.at_offset((y, x), (0, 0), FALSE),  // Right
            _ => unreachable!(),
        }
    };

    let k_max = h.max(w) + 1;

    for y in 0..h {
        for x in 0..w {
            let clue = problem[y][x];
            let (yi, xi) = (y as i32, x as i32);
            let lines = [
                border(yi, xi, 0),
                border(yi, xi, 1),
                border(yi, xi, 2),
                border(yi, xi, 3),
            ];
            let lcnt = count_true(lines.clone());

            // 各マスの次数: ループ上なら 2、そうでなければ 0
            // (数字・? いずれの丸マスも必ず 2。空きマスは full モードなら 2、それ以外は 0 か 2)
            if opts.loop_full {
                solver.add_expr(lcnt.clone().eq(2));
            } else if clue.is_some() {
                solver.add_expr(lcnt.clone().eq(2));
            } else {
                solver.add_expr(lcnt.clone().eq(0) | lcnt.clone().eq(2));
            }

            let clue = match clue {
                Some(clue) => clue,
                None => continue,
            };

            // s[d][k] = (セルから方向 d に k 本の辺が連続して線になっている)
            let s = solver.bool_var_2d((4, k_max));
            for k in 1..k_max {
                for (di, &(dy, dx)) in DIRECTIONS.iter().enumerate() {
                    // 方向 d に k 番目の辺 (k-1 個目と k 個目のセルの間)
                    let kth = border(yi + dy * (k as i32 - 1), xi + dx * (k as i32 - 1), di);
                    let prev = if k == 1 {
                        TRUE
                    } else {
                        s.at((di, k - 1)).expr()
                    };
                    solver.add_expr(s.at((di, k)).iff(prev & kth));
                }
            }
            let run_len = |d: usize| -> IntExpr {
                count_true(
                    (1..k_max)
                        .map(|k| s.at((d, k)).expr())
                        .collect::<Vec<BoolExpr>>(),
                )
            };

            // ループ上 (lcnt == 2) のとき、線になっている 2 方向の組に対して
            // 白丸: 長さが等しい / 黒丸: 長さが異なる / 数字: 長さの和 = 数字
            for a in 0..4 {
                for b in (a + 1)..4 {
                    let on = lines[a].clone() & lines[b].clone();
                    if clue.white {
                        solver.add_expr(on.clone().imp(run_len(a).eq(run_len(b))));
                    } else {
                        solver.add_expr(on.clone().imp(run_len(a).ne(run_len(b))));
                    }
                    if let Some(n) = clue.num {
                        solver.add_expr(on.imp((run_len(a) + run_len(b)).eq(n)));
                    }
                }
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(&edges))
}

fn encode_number16(v: i32, out: &mut String) {
    if v < 16 {
        out.push(char::from_digit(v as u32, 16).unwrap());
    } else if v < 256 {
        out.push('-');
        out.push_str(&format!("{:x}", v));
    } else if v < 4096 {
        out.push('+');
        out.push_str(&format!("{:x}", v));
    } else if v < 8192 {
        out.push('=');
        out.push_str(&format!("{:03x}", v - 4096));
    } else if v < 12240 {
        out.push('@');
        out.push_str(&format!("{:03x}", v - 8192));
    } else if v < 77776 {
        out.push('*');
        out.push_str(&format!("{:04x}", v - 12240));
    } else {
        out.push('$');
        out.push_str(&format!("{:05x}", v - 77776));
    }
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    serialize_problem_with_options(problem, &BalanceLoopOptions { loop_full: false })
}

pub fn serialize_problem_with_options(
    problem: &Problem,
    opts: &BalanceLoopOptions,
) -> Option<String> {
    let h = problem.len();
    let w = problem[0].len();

    let mut data = String::new();
    let mut count = 0usize;
    let flush = |data: &mut String, count: &mut usize| {
        while *count > 0 {
            let c = (*count).min(20);
            data.push(char::from_digit((15 + c) as u32, 36).unwrap());
            *count -= c;
        }
    };

    for row in problem {
        for cell in row {
            let v = match cell {
                None => {
                    count += 1;
                    continue;
                }
                Some(clue) => match clue.num {
                    None => {
                        if clue.white {
                            0
                        } else {
                            1
                        }
                    }
                    Some(n) => {
                        if clue.white {
                            2 * n
                        } else {
                            2 * n + 1
                        }
                    }
                },
            };
            flush(&mut data, &mut count);
            encode_number16(v, &mut data);
        }
    }
    flush(&mut data, &mut count);

    let flag = if opts.loop_full { "f/" } else { "" };
    Some(format!(
        "https://puzz.link/p?balance/{}{}/{}/{}",
        flag, w, h, data
    ))
}

/// readNumber16 相当: Some((値, 消費文字数))。'0'-'9','a'-'f' = 1 桁、'-' = 2 桁、
/// '+' = 3 桁、'=' = 3 桁 + 4096、'@' = 3 桁 + 8192、'*' = 4 桁 + 12240、'$' = 5 桁 + 77776。
fn read_number16(chars: &[char], i: usize) -> Option<(i32, usize)> {
    let ca = chars[i];
    let take = |n: usize| -> Option<i32> {
        let s: String = chars.get(i + 1..i + 1 + n)?.iter().collect();
        i32::from_str_radix(&s, 16).ok()
    };
    match ca {
        '0'..='9' | 'a'..='f' => Some((ca.to_digit(16).unwrap() as i32, 1)),
        '-' => Some((take(2)?, 3)),
        '+' => Some((take(3)?, 3)),
        '=' => Some((take(3)? + 4096, 4)),
        '@' => Some((take(3)? + 8192, 4)),
        '*' => Some((take(4)? + 12240, 5)),
        '$' => Some((take(5)? + 77776, 6)),
        _ => None,
    }
}

pub fn deserialize_problem(url: &str) -> Option<(Problem, BalanceLoopOptions)> {
    let body = strip_prefix(url)?;
    let mut parts = body.split('/');
    let kind = parts.next()?;
    if !["balance", "balanceloop", "balance-loop", "balance_loop"].contains(&kind) {
        return None;
    }
    // qdata -> [(pflag)/](cols)/(rows)/(bstr)
    let second = parts.next()?;
    let (loop_full, w, h) = if let Ok(wv) = second.parse::<usize>() {
        (false, wv, parts.next()?.parse().ok()?)
    } else {
        (
            second.contains('f'),
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
        )
    };
    let data = parts.next().unwrap_or("");

    if w < 1 || h < 1 {
        return None;
    }

    let mut problem = vec![vec![None; w]; h];
    let chars: Vec<char> = data.chars().collect();
    let mut c = 0usize;
    let mut i = 0usize;
    while i < chars.len() && c < h * w {
        let ca = chars[i];
        if ('g'..='z').contains(&ca) {
            c += ca.to_digit(36).unwrap() as usize - 15;
            i += 1;
        } else if ca == '.' {
            // '.' は空きマス扱い (UI の decodePzpr と同様)
            c += 1;
            i += 1;
        } else if let Some((v, len)) = read_number16(&chars, i) {
            let clue = match v {
                0 => Some(Clue {
                    white: true,
                    num: None,
                }),
                1 => Some(Clue {
                    white: false,
                    num: None,
                }),
                v if v % 2 == 0 => Some(Clue {
                    white: true,
                    num: Some(v / 2),
                }),
                v => Some(Clue {
                    white: false,
                    num: Some((v - 1) / 2),
                }),
            };
            problem[c / w][c % w] = clue;
            c += 1;
            i += len;
        } else {
            i += 1;
        }
    }
    Some((problem, BalanceLoopOptions { loop_full }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_3x3() -> Problem {
        // 外周ループ: (0,0) 白4, (0,1) 白2, (2,2) 白4。中央マスはループ外。
        let mut ret = vec![vec![None; 3]; 3];
        ret[0][0] = Some(Clue {
            white: true,
            num: Some(4),
        });
        ret[0][1] = Some(Clue {
            white: true,
            num: Some(2),
        });
        ret[2][2] = Some(Clue {
            white: true,
            num: Some(4),
        });
        ret
    }

    #[test]
    fn test_balance_loop_problem() {
        let problem = problem_3x3();
        let ans = solve_balance_loop(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();

        // 外周 8 辺がすべて線になり、中央マスは孤立する
        assert_eq!(ans.horizontal[0][0], Some(true));
        assert_eq!(ans.horizontal[0][1], Some(true));
        assert_eq!(ans.horizontal[1][0], Some(false));
        assert_eq!(ans.horizontal[1][1], Some(false));
        assert_eq!(ans.horizontal[2][0], Some(true));
        assert_eq!(ans.horizontal[2][1], Some(true));
        assert_eq!(ans.vertical[0][0], Some(true));
        assert_eq!(ans.vertical[1][0], Some(true));
        assert_eq!(ans.vertical[0][1], Some(false));
        assert_eq!(ans.vertical[1][1], Some(false));
        assert_eq!(ans.vertical[0][2], Some(true));
        assert_eq!(ans.vertical[1][2], Some(true));
    }

    #[test]
    fn test_balance_loop_question_cell_must_be_on_loop() {
        // 2x2: (0,0) 白? のみ。? マスも必ずループ上なので、唯一の解は 4 マスを回る小ループ。
        // (? がループ外でもよい旧実装だと「空ループ」も解になって交点が空になる)
        let mut problem = vec![vec![None; 2]; 2];
        problem[0][0] = Some(Clue {
            white: true,
            num: None,
        });
        let ans = solve_balance_loop(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        assert_eq!(ans.horizontal[0][0], Some(true));
        assert_eq!(ans.horizontal[1][0], Some(true));
        assert_eq!(ans.vertical[0][0], Some(true));
        assert_eq!(ans.vertical[0][1], Some(true));
    }

    #[test]
    fn test_balance_loop_full() {
        // 4x4 loop_full ('f'): ヘビ型ハミルトン閉路
        // (0,0) 黒4, (0,3) 白6, (3,3) 白6, (3,0) 黒4
        let mut problem = vec![vec![None; 4]; 4];
        problem[0][0] = Some(Clue {
            white: false,
            num: Some(4),
        });
        problem[0][3] = Some(Clue {
            white: true,
            num: Some(6),
        });
        problem[3][3] = Some(Clue {
            white: true,
            num: Some(6),
        });
        problem[3][0] = Some(Clue {
            white: false,
            num: Some(4),
        });
        let ans = solve_balance_loop_with_options(&problem, BalanceLoopOptions { loop_full: true });
        assert!(ans.is_some());
        let ans = ans.unwrap();

        let expected_h = vec![
            vec![true, true, true],
            vec![true, true, false],
            vec![true, true, false],
            vec![true, true, true],
        ];
        let expected_v = vec![
            vec![true, false, false, true],
            vec![false, false, true, true],
            vec![true, false, false, true],
        ];
        for y in 0..4 {
            for x in 0..3 {
                assert_eq!(ans.horizontal[y][x], Some(expected_h[y][x]));
            }
        }
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(ans.vertical[y][x], Some(expected_v[y][x]));
            }
        }
    }

    #[test]
    fn test_balance_loop_serializer() {
        let problem = problem_3x3();
        let encoded = serialize_problem(&problem).unwrap();
        assert_eq!(encoded, "https://puzz.link/p?balance/3/3/84l8");
        let (decoded, opts) = deserialize_problem(&encoded).unwrap();
        assert_eq!(decoded, problem);
        assert!(!opts.loop_full);

        // UI のテストデータと同じ URL をデコードできる
        let (decoded2, _) = deserialize_problem("https://puzz.link/p?balance/5/5/g9m1k0m8g").unwrap();
        assert_eq!(
            decoded2[0][1],
            Some(Clue {
                white: false,
                num: Some(4)
            })
        );
        assert_eq!(
            decoded2[1][4],
            Some(Clue {
                white: false,
                num: None
            })
        );
        assert_eq!(
            decoded2[3][0],
            Some(Clue {
                white: true,
                num: None
            })
        );
        assert_eq!(
            decoded2[4][3],
            Some(Clue {
                white: true,
                num: Some(4)
            })
        );
        assert_eq!(decoded2[2][2], None);
        assert_eq!(
            serialize_problem(&decoded2).unwrap(),
            "https://puzz.link/p?balance/5/5/g9m1k0m8g"
        );

        // ? マス・黒丸数字・loop_full フラグも往復できる
        let mut problem3 = vec![vec![None; 4]; 4];
        problem3[0][0] = Some(Clue {
            white: false,
            num: Some(4),
        });
        problem3[0][3] = Some(Clue {
            white: true,
            num: Some(6),
        });
        problem3[1][1] = Some(Clue {
            white: true,
            num: None,
        });
        problem3[2][3] = Some(Clue {
            white: false,
            num: None,
        });
        problem3[3][0] = Some(Clue {
            white: false,
            num: Some(4),
        });
        problem3[3][3] = Some(Clue {
            white: true,
            num: Some(6),
        });
        let encoded3 = serialize_problem_with_options(
            &problem3,
            &BalanceLoopOptions { loop_full: true },
        )
        .unwrap();
        let (decoded3, opts3) = deserialize_problem(&encoded3).unwrap();
        assert_eq!(decoded3, problem3);
        assert!(opts3.loop_full);
    }
}
