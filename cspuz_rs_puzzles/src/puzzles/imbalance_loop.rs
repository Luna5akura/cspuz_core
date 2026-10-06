use cspuz_rs::graph;
use cspuz_rs::serializer::strip_prefix;
use cspuz_rs::solver::{count_true, BoolExpr, Solver, FALSE, TRUE};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImbDir {
    Up,
    Down,
    Left,
    Right,
}

impl ImbDir {
    pub fn opposite(self) -> ImbDir {
        match self {
            ImbDir::Up => ImbDir::Down,
            ImbDir::Down => ImbDir::Up,
            ImbDir::Left => ImbDir::Right,
            ImbDir::Right => ImbDir::Left,
        }
    }

    // pzpr の qdir 定数と一致させる (UP=1, DN=2, LT=3, RT=4)
    pub fn to_code(self) -> i32 {
        match self {
            ImbDir::Up => 1,
            ImbDir::Down => 2,
            ImbDir::Left => 3,
            ImbDir::Right => 4,
        }
    }

    pub fn from_code(c: i32) -> Option<ImbDir> {
        match c {
            1 => Some(ImbDir::Up),
            2 => Some(ImbDir::Down),
            3 => Some(ImbDir::Left),
            4 => Some(ImbDir::Right),
            _ => None,
        }
    }

    fn is_horizontal(self) -> bool {
        self == ImbDir::Left || self == ImbDir::Right
    }
}

/// 手がかり: num = None は ? を表す。dir = None は矢印なし。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Clue {
    pub num: Option<i32>,
    pub dir: Option<ImbDir>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub black: bool,
    pub clue: Option<Clue>,
}

pub type Problem = Vec<Vec<Cell>>;

pub fn solve_imbalance_loop(problem: &Problem) -> Option<graph::BoolGridEdgesIrrefutableFacts> {
    let h = problem.len();
    let w = problem[0].len();

    let mut solver = Solver::new();
    // 盤面の内側の辺のみ (outer frame は含めない)
    let edges = graph::BoolGridEdges::new(&mut solver, (h - 1, w - 1));
    solver.add_answer_key_bool(&edges.horizontal);
    solver.add_answer_key_bool(&edges.vertical);
    graph::single_cycle_grid_edges(&mut solver, &edges);

    // セル (y, x) の dir 方向の辺 (盤外なら FALSE)
    let border = |y: i32, x: i32, d: ImbDir| -> BoolExpr {
        let (y, x) = (y as usize, x as usize);
        match d {
            ImbDir::Up => edges.vertical.at_offset((y, x), (-1, 0), FALSE),
            ImbDir::Down => edges.vertical.at_offset((y, x), (0, 0), FALSE),
            ImbDir::Left => edges.horizontal.at_offset((y, x), (0, -1), FALSE),
            ImbDir::Right => edges.horizontal.at_offset((y, x), (0, 0), FALSE),
        }
    };

    for y in 0..h {
        for x in 0..w {
            let cell = problem[y][x];
            let (y, x) = (y as i32, x as i32);
            if cell.black {
                solver.add_expr(!border(y, x, ImbDir::Up));
                solver.add_expr(!border(y, x, ImbDir::Down));
                solver.add_expr(!border(y, x, ImbDir::Left));
                solver.add_expr(!border(y, x, ImbDir::Right));
            } else {
                // 白マスはちょうど2辺が線になる (ループがこのマスを通る)
                solver.add_expr(
                    count_true([
                        border(y, x, ImbDir::Up),
                        border(y, x, ImbDir::Down),
                        border(y, x, ImbDir::Left),
                        border(y, x, ImbDir::Right),
                    ])
                    .eq(2),
                );
            }
        }
    }

    let k_max = h.max(w) + 1;

    for y in 0..h {
        for x in 0..w {
            let clue = match problem[y][x].clue {
                Some(clue) => clue,
                None => continue,
            };
            let (yi, xi) = (y as i32, x as i32);

            // s[d][k] = (セルから方向 d に k 本の辺が連続して線になっている)
            let s = solver.bool_var_2d((4, k_max));
            let dirs = [ImbDir::Up, ImbDir::Down, ImbDir::Left, ImbDir::Right];
            for k in 1..k_max {
                for (di, &d) in dirs.iter().enumerate() {
                    let (dy, dx) = match d {
                        ImbDir::Up => (-1, 0),
                        ImbDir::Down => (1, 0),
                        ImbDir::Left => (0, -1),
                        ImbDir::Right => (0, 1),
                    };
                    // 方向 d に k 番目の辺 (k-1 個目と k 個目のセルの間)
                    let kth = border(yi + dy * (k as i32 - 1), xi + dx * (k as i32 - 1), d);
                    let prev = if k == 1 {
                        TRUE
                    } else {
                        s.at((di, k - 1)).expr()
                    };
                    solver.add_expr(s.at((di, k)).iff(prev & kth));
                }
            }
            let run_len = |d: ImbDir| -> cspuz_rs::solver::IntExpr {
                let di = dirs.iter().position(|&e| e == d).unwrap();
                let arr = (1..k_max)
                    .map(|k| s.at((di, k)).expr())
                    .collect::<Vec<BoolExpr>>();
                count_true(arr)
            };

            let axis_h = border(yi, xi, ImbDir::Left) & border(yi, xi, ImbDir::Right);
            let axis_v = border(yi, xi, ImbDir::Up) & border(yi, xi, ImbDir::Down);

            match clue.dir {
                Some(d) => {
                    // 矢印の軸と一致し、矢印方向の長さ − 反対方向の長さ = 数字
                    let axis_d = if d.is_horizontal() { axis_h } else { axis_v };
                    solver.add_expr(axis_d);
                    let l_d = run_len(d);
                    let l_opp = run_len(d.opposite());
                    match clue.num {
                        Some(n) => solver.add_expr((l_d - l_opp).eq(n)),
                        None => solver.add_expr((l_d - l_opp).ge(1)),
                    }
                }
                None => {
                    // どちらかの軸で直進し、長さの差の絶対値が数字と一致
                    let l_up = run_len(ImbDir::Up);
                    let l_down = run_len(ImbDir::Down);
                    let l_left = run_len(ImbDir::Left);
                    let l_right = run_len(ImbDir::Right);
                    match clue.num {
                        Some(n) => {
                            solver.add_expr(
                                (axis_h
                                    & ((l_right.clone() - l_left.clone()).eq(n)
                                        | (l_left.clone() - l_right.clone()).eq(n)))
                                    | (axis_v
                                        & ((l_down.clone() - l_up.clone()).eq(n)
                                            | (l_up.clone() - l_down.clone()).eq(n))),
                            );
                        }
                        None => {
                            solver.add_expr(
                                (axis_h
                                    & ((l_right.clone() - l_left.clone()).ge(1)
                                        | (l_left.clone() - l_right.clone()).ge(1)))
                                    | (axis_v
                                        & ((l_down.clone() - l_up.clone()).ge(1)
                                            | (l_up.clone() - l_down.clone()).ge(1))),
                            );
                        }
                    }
                }
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(&edges))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let h = problem.len();
    let w = problem[0].len();

    let mut data = String::new();
    let mut count = 0usize;
    let flush = |data: &mut String, count: &mut usize| {
        while *count > 0 {
            let c = (*count).min(26);
            data.push(char::from_digit((c + 9) as u32, 36).unwrap());
            *count -= c;
        }
    };

    for y in 0..h {
        for x in 0..w {
            let cell = problem[y][x];
            let mut pstr = String::new();
            if cell.black {
                pstr.push('+');
            } else if let Some(clue) = cell.clue {
                let dir_code = clue.dir.map_or(0, |d| d.to_code());
                match clue.num {
                    Some(n) if (0..16).contains(&n) => {
                        pstr.push_str(&format!("{}{:x}", dir_code, n));
                    }
                    Some(n) if (16..256).contains(&n) => {
                        pstr.push_str(&format!("{}{:x}", dir_code + 5, n));
                    }
                    _ => {}
                }
                if pstr.is_empty() {
                    // 数字なしは ? として扱う
                    pstr.push_str(&format!("{}.", dir_code));
                }
            }
            if pstr.is_empty() {
                count += 1;
            } else {
                flush(&mut data, &mut count);
                data.push_str(&pstr);
            }
        }
    }
    flush(&mut data, &mut count);

    Some(format!(
        "https://puzz.link/p?imbalanceloop/{}/{}/{}",
        w, h, data
    ))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let body = strip_prefix(url)?;
    let mut parts = body.split('/');
    let kind = parts.next()?;
    if !["imbalanceloop", "imbalance-loop", "imbalance_loop"].contains(&kind) {
        return None;
    }
    let w: usize = parts.next()?.parse().ok()?;
    let h: usize = parts.next()?.parse().ok()?;
    let data = parts.next().unwrap_or("");

    if w < 1 || h < 1 {
        return None;
    }

    let mut problem = vec![
        vec![
            Cell {
                black: false,
                clue: None
            };
            w
        ];
        h
    ];
    let chars: Vec<char> = data.chars().collect();
    let mut c = 0usize;
    let mut i = 0usize;
    while i < chars.len() && c < h * w {
        let ca = chars[i];
        if ca == '+' {
            problem[c / w][c % w].black = true;
            c += 1;
        } else if ('0'..='4').contains(&ca) {
            let dir = ImbDir::from_code(ca.to_digit(16)? as i32);
            let next = *chars.get(i + 1)?;
            let num = if next == '.' {
                None
            } else {
                Some(next.to_digit(16)? as i32)
            };
            problem[c / w][c % w].clue = Some(Clue { num, dir });
            c += 1;
            i += 1;
        } else if ('5'..='9').contains(&ca) {
            let dir = ImbDir::from_code(ca.to_digit(16)? as i32 - 5);
            let num = chars.get(i + 1)?.to_digit(16)? as i32 * 16
                + chars.get(i + 2)?.to_digit(16)? as i32;
            problem[c / w][c % w].clue = Some(Clue {
                num: Some(num),
                dir,
            });
            c += 1;
            i += 2;
        } else if ('a'..='z').contains(&ca) {
            c += ca.to_digit(36)? as usize - 10;
            c += 1;
        }
        i += 1;
    }
    Some(problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 4x4 に (3,1) の手がかり「1」(矢印なし):
    // 右端のマスなのでタテに直進せざるを得ず、上1・下2 (差1) が唯一の解
    fn problem_for_tests() -> Problem {
        let mut ret = vec![
            vec![
                Cell {
                    black: false,
                    clue: None
                };
                4
            ];
            4
        ];
        ret[1][3].clue = Some(Clue {
            num: Some(1),
            dir: None,
        });
        ret
    }

    #[test]
    fn test_imbalance_loop_problem() {
        let problem = problem_for_tests();
        let ans = solve_imbalance_loop(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();

        // (3,0)-(3,1), (3,1)-(3,2), (3,2)-(3,3) は線になる
        // (手がかりをタテに直進し、上1・下2 で差が1)
        assert_eq!(ans.vertical[0][3], Some(true));
        assert_eq!(ans.vertical[1][3], Some(true));
        assert_eq!(ans.vertical[2][3], Some(true));
    }

    #[test]
    fn test_imbalance_loop_serializer() {
        let problem = problem_for_tests();
        let encoded = serialize_problem(&problem).unwrap();
        assert_eq!(encoded, "https://puzz.link/p?imbalanceloop/4/4/g01h");
        let decoded = deserialize_problem(&encoded).unwrap();
        assert_eq!(decoded, problem);

        // 矢印・黒マス・? も往復できる
        let mut problem2 = problem_for_tests();
        problem2[2][2].clue = Some(Clue {
            num: Some(2),
            dir: Some(ImbDir::Right),
        });
        problem2[1][1].black = true;
        problem2[3][0].clue = Some(Clue {
            num: None,
            dir: None,
        });
        let encoded2 = serialize_problem(&problem2).unwrap();
        let decoded2 = deserialize_problem(&encoded2).unwrap();
        assert_eq!(decoded2, problem2);
    }
}










