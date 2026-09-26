use cspuz_rs::solver::{count_true, Solver};

// Battleships (このフォークの statuepark ベースの実装)
//
// ルール:
//   盤面に艦隊 (バンクのピース) をすべて、ちょうど1回ずつ配置する。
//   異なる艦はタテヨコナナメに接しない。
//   盤面外の数字は、その列で艦の入るマスの数を表す。
//   一部のマスには艦の部品 (向き付きヒント) が与えられている。
//   波のマス (0) には艦を置けない。
//
// Problem = (clues, row_counts, col_counts, pieces)
//   clues[y][x]: -1 = 空きマス, 0 = 波マス (艦を置けない),
//                1-10 = 艦の部品ヒント
//                  (1:上向き=下が続く 2:下向き 3:左向き 4:右向き
//                   5:中央 6:単マス 7:左下カド 8:右下カド 9:左上カド 10:右上カド)
//   row_counts / col_counts: 盤面外の数字 (-1 = 制約なし)
//   pieces: 艦隊のピース一覧
pub type Problem = (Vec<Vec<i8>>, Vec<i32>, Vec<i32>, Vec<Vec<Vec<bool>>>);

fn from_hex(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'f' => Some((c - b'a' + 10) as i32),
        _ => None,
    }
}

// pzprの number16 エンコーディングをデコードする。
// 値: 0-15 = 1文字, 16-255 = "-" + 2文字, 256-4095 = "+" + 3文字,
// 4096-8191 = "=" + 3文字, 8192-12239 = "@" + 3文字,
// 12240-77775 = "*" + 4文字, 77776以上 = "$" + 5文字,
// -2 = ".", -1 = "g"-"z" の連続 (1-20個)。
fn parse_number16(input: &str, n: usize) -> Option<(Vec<i32>, usize)> {
    let bytes = input.as_bytes();
    let mut vals = vec![];
    let mut i = 0;
    while i < bytes.len() && vals.len() < n {
        let ca = bytes[i];
        if (b'0'..=b'9').contains(&ca) || (b'a'..=b'f').contains(&ca) {
            vals.push(from_hex(ca)?);
            i += 1;
        } else if ca == b'-' {
            let hi = from_hex(*bytes.get(i + 1)?)?;
            let lo = from_hex(*bytes.get(i + 2)?)?;
            vals.push(hi * 16 + lo);
            i += 3;
        } else if ca == b'+' {
            let a = from_hex(*bytes.get(i + 1)?)?;
            let b = from_hex(*bytes.get(i + 2)?)?;
            let c = from_hex(*bytes.get(i + 3)?)?;
            vals.push(a * 256 + b * 16 + c);
            i += 4;
        } else if ca == b'=' {
            let a = from_hex(*bytes.get(i + 1)?)?;
            let b = from_hex(*bytes.get(i + 2)?)?;
            let c = from_hex(*bytes.get(i + 3)?)?;
            vals.push(a * 256 + b * 16 + c + 4096);
            i += 4;
        } else if ca == b'@' || ca == b'%' {
            let a = from_hex(*bytes.get(i + 1)?)?;
            let b = from_hex(*bytes.get(i + 2)?)?;
            let c = from_hex(*bytes.get(i + 3)?)?;
            vals.push(a * 256 + b * 16 + c + 8192);
            i += 4;
        } else if ca == b'*' {
            let mut v = 0;
            for k in 1..=4 {
                v = v * 16 + from_hex(*bytes.get(i + k)?)?;
            }
            vals.push(v + 12240);
            i += 5;
        } else if ca == b'$' {
            let mut v = 0;
            for k in 1..=5 {
                v = v * 16 + from_hex(*bytes.get(i + k)?)?;
            }
            vals.push(v + 77776);
            i += 6;
        } else if ca == b'.' {
            vals.push(-2);
            i += 1;
        } else if (b'g'..=b'z').contains(&ca) {
            let run = (ca - b'a' + 10) - 15;
            for _ in 0..run {
                vals.push(-1);
            }
            i += 1;
        } else {
            return None;
        }
    }
    if vals.len() != n {
        return None;
    }
    Some((vals, i))
}

// バンクのピース ("wh+base32ビット" または "w:0/1文字列") をデコードする
fn deserialize_piece(str: &str) -> Option<Vec<Vec<bool>>> {
    if str.contains(':') {
        let mut it = str.split(':');
        let w: usize = it.next()?.parse().ok()?;
        let s = it.next()?;
        let h = s.len() / w;
        let mut ret = vec![vec![false; w]; h];
        for (i, ch) in s.bytes().enumerate() {
            if i < w * h {
                ret[i / w][i % w] = ch == b'1';
            }
        }
        Some(ret)
    } else {
        let bytes = str.as_bytes();
        if bytes.len() < 3 {
            return None;
        }
        let w = (bytes[0] as char).to_digit(36)? as usize;
        let h = (bytes[1] as char).to_digit(36)? as usize;
        let mut bits = vec![];
        for &c in &bytes[2..] {
            let v = (c as char).to_digit(32)? as u32;
            for k in (0..5).rev() {
                bits.push(((v >> k) & 1) == 1);
            }
        }
        let mut ret = vec![vec![false; w]; h];
        for i in 0..(w * h).min(bits.len()) {
            ret[i / w][i % w] = bits[i];
        }
        Some(ret)
    }
}

// バンクのプリセット (shortkey -> 艦隊のピース一覧)
fn preset_pieces(shortkey: &str) -> Option<Vec<&'static str>> {
    match shortkey {
        "c" => Some(vec!["11g", "11g", "11g", "21o", "21o", "31s"]),
        "d" => Some(vec![
            "11g", "11g", "11g", "11g", "21o", "21o", "21o", "31s", "31s", "41u",
        ]),
        "e" => Some(vec![
            "11g", "11g", "11g", "11g", "11g", "21o", "21o", "21o", "21o", "31s", "31s", "31s",
            "41u", "41u", "51v",
        ]),
        _ => None,
    }
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    // URL: .../battleships/w/h/<number16(excell+cell)>/<bank>
    //   bank: "/<count>/<piece>/..." または プリセットの "//<shortkey>"
    let body = url.split("battleships/").nth(1)?;
    let mut segs = body.split('/');
    let w: usize = segs.next()?.parse().ok()?;
    let h: usize = segs.next()?.parse().ok()?;
    let data = segs.next()?;

    let mut pieces = vec![];
    let bank_head = segs.next()?;
    if bank_head.is_empty() {
        // プリセット形式 ("//<shortkey>")
        let shortkey = segs.next()?;
        for p in preset_pieces(shortkey)? {
            pieces.push(deserialize_piece(p)?);
        }
    } else {
        let count: usize = bank_head.parse().ok()?;
        for _ in 0..count {
            pieces.push(deserialize_piece(segs.next()?)?);
        }
    }

    // excell (上=列の数字 w個, 左=行の数字 h個) + セル
    let (excell_vals, used) = parse_number16(data, w + h)?;
    let (cell_vals, _) = parse_number16(&data[used..], h * w)?;

    let mut col_counts = vec![0; w];
    let mut row_counts = vec![0; h];
    for x in 0..w {
        col_counts[x] = excell_vals[x];
    }
    for y in 0..h {
        row_counts[y] = excell_vals[w + y];
    }

    let mut clues = vec![vec![0i8; w]; h];
    for i in 0..h * w {
        clues[i / w][i % w] = cell_vals[i] as i8;
    }
    Some((clues, row_counts, col_counts, pieces))
}

fn enumerate_piece_transformations(piece: &[Vec<bool>]) -> Vec<Vec<Vec<bool>>> {
    fn rotate_piece_90(piece: &[Vec<bool>]) -> Vec<Vec<bool>> {
        let h = piece.len();
        let w = piece[0].len();
        let mut ret = vec![vec![false; h]; w];
        for i in 0..h {
            for j in 0..w {
                ret[j][h - i - 1] = piece[i][j];
            }
        }
        ret
    }

    fn flip_piece(piece: &[Vec<bool>]) -> Vec<Vec<bool>> {
        let h = piece.len();
        let w = piece[0].len();
        let mut ret = vec![vec![false; w]; h];
        for i in 0..h {
            for j in 0..w {
                ret[i][w - j - 1] = piece[i][j];
            }
        }
        ret
    }

    let mut piece = piece.to_vec();
    let mut ret = vec![];
    for _ in 0..4 {
        ret.push(piece.clone());
        ret.push(flip_piece(&piece));
        piece = rotate_piece_90(&piece);
    }

    let mut ret = ret
        .into_iter()
        .map(|p| {
            let mut miny = p.len();
            let mut minx = p[0].len();
            for (y, row) in p.iter().enumerate() {
                for (x, &b) in row.iter().enumerate() {
                    if b {
                        miny = miny.min(y);
                        minx = minx.min(x);
                    }
                }
            }
            let mut norm = vec![vec![false; p[0].len()]; p.len()];
            for y in 0..p.len() {
                for x in 0..p[0].len() {
                    if p[y][x] {
                        norm[y - miny][x - minx] = true;
                    }
                }
            }
            norm
        })
        .collect::<Vec<_>>();
    ret.sort();
    ret.dedup();
    ret
}

pub struct BattleshipsSolveResult {
    pub covered: Vec<Vec<Option<bool>>>,
    pub is_unique: bool,
}

pub fn solve_battleships(problem: &Problem) -> Option<BattleshipsSolveResult> {
    let (clues, row_counts, col_counts, pieces) = problem;
    let h = clues.len();
    let w = clues[0].len();
    let k = pieces.len();

    // 各ピースの配置一覧 (波マス(qnum=0)には置けない)
    let placements: Vec<Vec<Vec<(usize, usize)>>> = pieces
        .iter()
        .map(|piece| {
            let mut ret = vec![];
            for t in enumerate_piece_transformations(piece) {
                let ph = t.len();
                let pw = t[0].len();
                if ph > h || pw > w {
                    continue;
                }
                let cells = t
                    .iter()
                    .enumerate()
                    .flat_map(|(y, row)| {
                        row.iter()
                            .enumerate()
                            .filter_map(move |(x, &b)| if b { Some((y, x)) } else { None })
                    })
                    .collect::<Vec<_>>();

                for y in 0..=(h - ph) {
                    for x in 0..=(w - pw) {
                        if cells
                            .iter()
                            .all(|&(dy, dx)| clues[y + dy][x + dx] != 0)
                        {
                            ret.push(
                                cells
                                    .iter()
                                    .map(|&(dy, dx)| (y + dy, x + dx))
                                    .collect::<Vec<_>>(),
                            );
                        }
                    }
                }
            }
            ret
        })
        .collect();

    let mut solver = Solver::new();

    // 各ピースはちょうど1つの配置
    let placed: Vec<cspuz_rs::solver::BoolVarArray1D> = placements
        .iter()
        .map(|pl| solver.bool_var_1d(pl.len()))
        .collect();
    for p in 0..k {
        solver.add_expr(count_true(placed[p].clone()).eq(1));
        solver.add_answer_key_bool(&placed[p]);
    }

    // 各マスの被覆と、どのピースが覆うか
    let covered = &solver.bool_var_2d((h, w));
    let id = &solver.int_var_2d((h, w), 0, k as i32);
    solver.add_answer_key_bool(covered);
    for y in 0..h {
        for x in 0..w {
            let mut covering = vec![];
            let mut covering_p = vec![];
            for p in 0..k {
                let cp = placements[p]
                    .iter()
                    .enumerate()
                    .filter_map(|(i, cells)| {
                        if cells.contains(&(y, x)) {
                            Some(placed[p].at(i))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                for c in cp {
                    covering.push(c);
                }
                covering_p.push(count_true(
                    placements[p]
                        .iter()
                        .enumerate()
                        .filter_map(|(i, cells)| {
                            if cells.contains(&(y, x)) {
                                Some(placed[p].at(i))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                ));
            }
            solver.add_expr(count_true(covering.clone()).le(1));
            solver.add_expr(covered.at((y, x)).iff(count_true(covering).eq(1)));
            for p in 0..k {
                solver.add_expr(
                    id.at((y, x))
                        .eq(p as i32 + 1)
                        .iff(covering_p[p].clone().eq(1)),
                );
            }
            solver.add_expr(id.at((y, x)).eq(0).iff(!covered.at((y, x))));
        }
    }

    // 異なる艦はタテヨコナナメに接しない
    // (隣接する2マスが両方覆われているなら同じ艦)
    for y in 0..h {
        for x in 0..w {
            for &(dy, dx) in &[(0i32, 1i32), (1, 0), (1, 1), (1, -1)] {
                let ny = y as i32 + dy;
                let nx = x as i32 + dx;
                if ny < 0 || nx < 0 || ny >= h as i32 || nx >= w as i32 {
                    continue;
                }
                let (ny, nx) = (ny as usize, nx as usize);
                solver.add_expr(
                    (!covered.at((y, x)) | !covered.at((ny, nx)))
                        | id.at((y, x)).eq(id.at((ny, nx))),
                );
            }
        }
    }

    // ヒントマス・波マス
    let top = |y: usize, x: usize| -> cspuz_rs::solver::BoolExpr {
        if y > 0 {
            covered.at((y - 1, x)).expr()
        } else {
            cspuz_rs::solver::FALSE
        }
    };
    let bottom = |y: usize, x: usize| -> cspuz_rs::solver::BoolExpr {
        if y + 1 < h {
            covered.at((y + 1, x)).expr()
        } else {
            cspuz_rs::solver::FALSE
        }
    };
    let left = |y: usize, x: usize| -> cspuz_rs::solver::BoolExpr {
        if x > 0 {
            covered.at((y, x - 1)).expr()
        } else {
            cspuz_rs::solver::FALSE
        }
    };
    let right = |y: usize, x: usize| -> cspuz_rs::solver::BoolExpr {
        if x + 1 < w {
            covered.at((y, x + 1)).expr()
        } else {
            cspuz_rs::solver::FALSE
        }
    };

    for y in 0..h {
        for x in 0..w {
            match clues[y][x] {
                0 => {
                    solver.add_expr(!covered.at((y, x)));
                }
                1 => {
                    // 上向き (下に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(!top(y, x));
                    solver.add_expr(bottom(y, x));
                    solver.add_expr(!left(y, x));
                    solver.add_expr(!right(y, x));
                }
                2 => {
                    // 下向き (上に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(top(y, x));
                    solver.add_expr(!bottom(y, x));
                    solver.add_expr(!left(y, x));
                    solver.add_expr(!right(y, x));
                }
                3 => {
                    // 左向き (右に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(!top(y, x));
                    solver.add_expr(!bottom(y, x));
                    solver.add_expr(!left(y, x));
                    solver.add_expr(right(y, x));
                }
                4 => {
                    // 右向き (左に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(!top(y, x));
                    solver.add_expr(!bottom(y, x));
                    solver.add_expr(left(y, x));
                    solver.add_expr(!right(y, x));
                }
                5 => {
                    // 中央 (タテかヨコに両側へ続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(
                        (top(y, x) & bottom(y, x) & !left(y, x) & !right(y, x))
                            | (!top(y, x) & !bottom(y, x) & left(y, x) & right(y, x)),
                    );
                }
                6 => {
                    // 単マス
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(!top(y, x));
                    solver.add_expr(!bottom(y, x));
                    solver.add_expr(!left(y, x));
                    solver.add_expr(!right(y, x));
                }
                7 => {
                    // 左下カド (下と右に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(!top(y, x));
                    solver.add_expr(bottom(y, x));
                    solver.add_expr(!left(y, x));
                    solver.add_expr(right(y, x));
                }
                8 => {
                    // 右下カド (下と左に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(!top(y, x));
                    solver.add_expr(bottom(y, x));
                    solver.add_expr(left(y, x));
                    solver.add_expr(!right(y, x));
                }
                9 => {
                    // 左上カド (上と右に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(top(y, x));
                    solver.add_expr(!bottom(y, x));
                    solver.add_expr(!left(y, x));
                    solver.add_expr(right(y, x));
                }
                10 => {
                    // 右上カド (上と左に続く)
                    solver.add_expr(covered.at((y, x)));
                    solver.add_expr(top(y, x));
                    solver.add_expr(!bottom(y, x));
                    solver.add_expr(left(y, x));
                    solver.add_expr(!right(y, x));
                }
                _ => (),
            }
        }
    }

    // 盤面外の数字
    for y in 0..h {
        if row_counts[y] >= 0 {
            solver.add_expr(
                count_true((0..w).map(|x| covered.at((y, x))).collect::<Vec<_>>())
                    .eq(row_counts[y]),
            );
        }
    }
    for x in 0..w {
        if col_counts[x] >= 0 {
            solver.add_expr(
                count_true((0..h).map(|y| covered.at((y, x))).collect::<Vec<_>>())
                    .eq(col_counts[x]),
            );
        }
    }

    solver.irrefutable_facts().map(|f| {
        let covered_cells = f.get(covered);
        // 解の一意性は盤面 (被覆) の一意性で判定する
        // (同一形状のピースの割り当ての入れ替えは別解と数えない)
        let is_unique = covered_cells.iter().flatten().all(|v| v.is_some());
        BattleshipsSolveResult {
            covered: covered_cells,
            is_unique,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battleships_url() {
        // 5x1: 2マス艦2隻
        let url = "https://puzz.link/p?battleships/5/1/lk/2/21o/21o";
        let problem = deserialize_problem(url).unwrap();
        assert_eq!(problem.0, vec![vec![-1, -1, -1, -1, -1]]);
        assert_eq!(problem.1, vec![-1]);
        assert_eq!(problem.2, vec![-1, -1, -1, -1, -1]);
        assert_eq!(problem.3.len(), 2);
        assert_eq!(problem.3[0], vec![vec![true, true]]);
    }

    #[test]
    fn test_battleships_unique() {
        // 5x1: 2マス艦2隻 → 唯一解 ({(0,0),(0,1)} と {(0,3),(0,4)})
        let url = "https://puzz.link/p?battleships/5/1/lk/2/21o/21o";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_battleships(&problem).unwrap();
        assert!(ans.is_unique);
        for x in 0..2 {
            assert_eq!(ans.covered[0][x], Some(true));
        }
        assert_eq!(ans.covered[0][2], Some(false));
        for x in 3..5 {
            assert_eq!(ans.covered[0][x], Some(true));
        }
    }

    #[test]
    fn test_battleships_water_and_clue() {
        // 3x3: 単マス艦1隻、ヒント(6:単マス)が(0,0)、残りは波マス
        let url = "https://puzz.link/p?battleships/3/3/100100600000000/1/11g";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_battleships(&problem).unwrap();
        assert!(ans.is_unique);
        assert_eq!(ans.covered[0][0], Some(true));
        assert_eq!(ans.covered[0][1], Some(false));
        assert_eq!(ans.covered[1][0], Some(false));
    }

    #[test]
    fn test_battleships_unsolvable() {
        // 2x2: 2マス艦2隻 + 全列2 → 隣接禁止のため解なし
        let url = "https://puzz.link/p?battleships/2/2/2222j/2/21o/21o";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_battleships(&problem);
        assert!(ans.is_none());
    }
}
