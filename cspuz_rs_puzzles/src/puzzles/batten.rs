use cspuz_rs::solver::{count_true, Solver};

/// Battenberg Painting (batten): shade cells so that:
///
/// 1. Around a marked intersection (the Battenburg symbol), the four cells
///    form a checkerboard pattern: exactly two shaded cells, diagonally
///    opposite each other.
/// 2. Around an unmarked intersection, the four cells must NOT form a
///    checkerboard pattern.
/// 3. A number on the outside of a row or column is the number of shaded
///    cells in that row or column.
///
/// Problem data:
/// - `row_clues[r]`: shaded-cell count clue of row r (None = no clue).
/// - `col_clues[c]`: shaded-cell count clue of column c (None = no clue).
/// - `marks[y][x]`: whether the intersection between cells (y,x), (y,x+1),
///   (y+1,x), (y+1,x+1) is marked.
pub type Problem = (Vec<Option<i32>>, Vec<Option<i32>>, Vec<Vec<bool>>);

pub fn solve_batten(problem: &Problem) -> Option<(Vec<Vec<Option<bool>>>, bool)> {
    let (row_clues, col_clues, marks) = problem;
    let h = row_clues.len();
    let w = col_clues.len();
    if h == 0 || w == 0 {
        return None;
    }
    if marks.len() != h - 1 || marks.iter().any(|r| r.len() != w - 1) {
        return None;
    }

    // Build the constraints twice: first to extract irrefutable facts
    // (cells that are shaded in every answer), then to enumerate answers.
    let facts = {
        let mut s = Solver::new();
        let shaded = add_constraints(&mut s, row_clues, col_clues, marks);
        match s.irrefutable_facts() {
            Some(f) => f.get(&shaded),
            None => vec![],
        }
    };
    let mut solver = Solver::new();
    let shaded = add_constraints(&mut solver, row_clues, col_clues, marks);
    const MAX_ENUM: usize = 5000;
    let answers: Vec<_> = solver.answer_iter().take(MAX_ENUM).collect();
    if answers.is_empty() {
        return None;
    }
    let truncated = answers.len() >= MAX_ENUM;
    if truncated {
        return Some((facts, true));
    }
    let mut out = vec![vec![None; w]; h];
    for y in 0..h {
        for x in 0..w {
            let v = answers[0].get_unwrap(&shaded.at((y, x)));
            if answers.iter().all(|a| a.get_unwrap(&shaded.at((y, x))) == v) {
                out[y][x] = Some(v);
            }
        }
    }
    Some((out, truncated))
}

fn add_constraints(
    solver: &mut Solver,
    row_clues: &[Option<i32>],
    col_clues: &[Option<i32>],
    marks: &[Vec<bool>],
) -> cspuz_rs::solver::BoolVarArray2D {
    let h = row_clues.len();
    let w = col_clues.len();
    let shaded = solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(&shaded);

    // Row/column shaded-count clues.
    for r in 0..h {
        if let Some(n) = row_clues[r] {
            if n >= 0 {
                let exprs: Vec<_> = (0..w).map(|x| shaded.at((r, x)).expr()).collect();
                solver.add_expr(count_true(exprs).eq(n));
            }
        }
    }
    for c in 0..w {
        if let Some(n) = col_clues[c] {
            if n >= 0 {
                let exprs: Vec<_> = (0..h).map(|y| shaded.at((y, c)).expr()).collect();
                solver.add_expr(count_true(exprs).eq(n));
            }
        }
    }

    // Checkerboard constraints around the inner intersections.
    for y in 0..h - 1 {
        for x in 0..w - 1 {
            let nw = shaded.at((y, x)).expr();
            let ne = shaded.at((y, x + 1)).expr();
            let sw = shaded.at((y + 1, x)).expr();
            let se = shaded.at((y + 1, x + 1)).expr();
            if marks[y][x] {
                // Exactly two shaded cells, diagonally opposite:
                // nw != ne, nw == se, ne == sw.
                solver.add_expr(
                    (nw.clone() ^ ne.clone()) & nw.clone().iff(se.clone()) & ne.iff(sw),
                );
            } else {
                // Not a checkerboard: nw != se or ne != sw or nw == ne.
                solver.add_expr((nw.clone() ^ se.clone()) | (ne.clone() ^ sw.clone()) | nw.iff(ne));
            }
        }
    }

    shaded
}

fn hex_digit(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'f' => Some((c - b'a' + 10) as i32),
        b'A'..=b'F' => Some((c - b'A' + 10) as i32),
        _ => None,
    }
}

fn base36(v: usize) -> String {
    char::from_digit(v as u32, 36)
        .expect("count must be at most 35")
        .to_string()
}

fn flush_skips(payload: &mut String, skipped: &mut usize) {
    while *skipped > 0 {
        let chunk = (*skipped).min(20);
        payload.push((b'g' + chunk as u8 - 1) as char);
        *skipped -= chunk;
    }
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (row_clues, col_clues, marks) = problem;
    let h = row_clues.len();
    let w = col_clues.len();
    if h == 0 || w == 0 {
        return None;
    }
    if marks.len() != h - 1 || marks.iter().any(|r| r.len() != w - 1) {
        return None;
    }

    let mut payload = String::new();

    // Cross marks: run-length (base 36) over the inner intersections.
    // Mirrors pzprjs Encode.encodeCrossMark.
    let n_cross = (h - 1) * (w - 1);
    if n_cross == 0 {
        payload.push('0');
    } else {
        let mut count = 0usize;
        for y in 0..h - 1 {
            for x in 0..w - 1 {
                if marks[y][x] {
                    payload.push_str(&base36(count));
                    count = 0;
                } else {
                    count += 1;
                    if count == 36 {
                        payload.push('.');
                        count = 0;
                    }
                }
            }
        }
        if count > 0 || payload.is_empty() {
            payload.push_str(&base36(count));
        }
    }

    // Excell clues: top clues (columns) then left clues (rows).
    let mut skipped = 0usize;
    for &v in col_clues.iter().chain(row_clues.iter()) {
        match v {
            None => skipped += 1,
            Some(n) if (0..16).contains(&n) => {
                flush_skips(&mut payload, &mut skipped);
                payload.push(char::from_digit(n as u32, 16)?);
            }
            Some(n) if (0..256).contains(&n) => {
                flush_skips(&mut payload, &mut skipped);
                payload.push('-');
                payload.push_str(&format!("{:02x}", n));
            }
            _ => return None,
        }
    }
    flush_skips(&mut payload, &mut skipped);

    Some(format!("https://puzz.link/p?batten/{w}/{h}/{payload}"))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let body = url.split('?').nth(1)?;
    let mut parts = body.split('/');
    if parts.next()? != "batten" {
        return None;
    }
    let w: usize = parts.next()?.parse().ok()?;
    let h: usize = parts.next()?.parse().ok()?;
    let payload = parts.next()?;
    let bytes = payload.as_bytes();
    let mut pos = 0usize;

    // Cross marks over the (h-1) x (w-1) inner intersections.
    let n_cross = (h - 1) * (w - 1);
    let mut marks = vec![vec![false; w - 1]; h - 1];
    if n_cross == 0 {
        if pos >= bytes.len() {
            return None;
        }
        pos += 1;
    } else {
        let mut cc = 0usize;
        while pos < bytes.len() && cc < n_cross {
            let c = bytes[pos];
            if c == b'.' {
                cc += 36;
                pos += 1;
                continue;
            }
            let v = if (b'0'..=b'9').contains(&c) {
                (c - b'0') as usize
            } else if (b'a'..=b'z').contains(&c) {
                (c - b'a' + 10) as usize
            } else {
                return None;
            };
            cc += v;
            if cc < n_cross {
                marks[cc / (w - 1)][cc % (w - 1)] = true;
            }
            cc += 1;
            pos += 1;
        }
    }

    // Excell clues: top clues (columns, w entries) then left clues (rows).
    let n_excell = w + h;
    let mut values: Vec<Option<i32>> = vec![None; n_excell];
    let mut ec = 0usize;
    while pos < bytes.len() && ec < n_excell {
        let c = bytes[pos];
        if (b'0'..=b'9').contains(&c) || (b'a'..=b'f').contains(&c) {
            values[ec] = Some(hex_digit(c)?);
            pos += 1;
        } else if c == b'-' {
            if pos + 2 >= bytes.len() {
                return None;
            }
            values[ec] = Some(hex_digit(bytes[pos + 1])? * 16 + hex_digit(bytes[pos + 2])?);
            pos += 3;
        } else if c == b'.' {
            // "?" clue: no constraint.
            pos += 1;
        } else if (b'g'..=b'z').contains(&c) {
            ec += (c - b'g' + 1) as usize;
            pos += 1;
            continue;
        } else {
            return None;
        }
        ec += 1;
    }
    let col_clues = values[..w].to_vec();
    let row_clues = values[w..].to_vec();
    Some((row_clues, col_clues, marks))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 5x5 board from pzprjs test/script/batten.js, together with its
    /// known-valid answer (# = shaded, + = decided unshaded).
    pub(super) fn test_problem() -> Problem {
        let row_clues = vec![None, Some(3), None, None, Some(3)];
        let col_clues = vec![Some(2), None, Some(2), None, Some(2)];
        let marks = vec![
            vec![true, false, false, false],
            vec![false, false, false, false],
            vec![true, false, false, true],
            vec![false, false, false, true],
        ];
        (row_clues, col_clues, marks)
    }

    pub(super) fn known_answer() -> Vec<Vec<bool>> {
        // "#" cells from the null failcheck entry.
        let rows = [
            ".###.",
            "#..##",
            "#..#.",
            ".#..#",
            ".###.",
        ];
        rows.iter()
            .map(|r| r.chars().map(|c| c == '#').collect())
            .collect()
    }

    #[test]
    fn test_batten_serializer() {
        let url = "https://puzz.link/p?batten/5/5/07232g2g2g3h3";
        let decoded = deserialize_problem(url).unwrap();
        assert_eq!(decoded, test_problem());
        assert_eq!(serialize_problem(&decoded).unwrap(), url);
    }

    #[test]
    fn test_batten_solver_runs() {
        let problem = test_problem();
        let ans = solve_batten(&problem);
        assert!(ans.is_some(), "the test puzzle must be solvable");
    }

    #[test]
    fn test_batten_known_answer_valid() {
        // The known-valid pzprjs answer must be a model of the constraints.
        let problem = test_problem();
        let (row_clues, col_clues, marks) = &problem;
        let (h, w) = (row_clues.len(), col_clues.len());
        let ans = known_answer();
        let mut solver = Solver::new();
        let shaded = add_constraints(&mut solver, row_clues, col_clues, marks);
        for y in 0..h {
            for x in 0..w {
                if ans[y][x] {
                    solver.add_expr(shaded.at((y, x)));
                } else {
                    solver.add_expr(!shaded.at((y, x)));
                }
            }
        }
        assert!(solver.solve().is_some(), "the known answer must satisfy all constraints");
    }

    #[test]
    fn test_batten_invalid_boards_rejected() {
        // shNoDiag: a marked intersection does not have a checkerboard.
        let row_clues = vec![None, Some(3), None, None, Some(3)];
        let col_clues = vec![Some(2), None, Some(2), None, Some(2)];
        let marks = test_problem().2;
        // Answer: ..##. / 3##..# / #...# / .#.#. / 3.###.
        let rows = [
            "..##.",
            "##..#",
            "#...#",
            ".#.#.",
            ".###.",
        ];
        let ans: Vec<Vec<bool>> = rows
            .iter()
            .map(|r| r.chars().map(|c| c == '#').collect())
            .collect();
        check_answer_conflict(&row_clues, &col_clues, &marks, &ans, false);

        // shDiag: an unmarked intersection forms a checkerboard.
        let rows = [
            "#...#",
            ".###.",
            ".#.#.",
            "#...#",
            ".###.",
        ];
        let ans: Vec<Vec<bool>> = rows
            .iter()
            .map(|r| r.chars().map(|c| c == '#').collect())
            .collect();
        check_answer_conflict(&row_clues, &col_clues, &marks, &ans, false);

        // exShadeNe: a row/column clue count does not match.
        let rows = [
            "..#.#.",
            "#....",
            "#...#",
            ".###.",
            ".##.#",
        ];
        let ans: Vec<Vec<bool>> = rows
            .iter()
            .map(|r| r.chars().map(|c| c == '#').collect())
            .collect();
        check_answer_conflict(&row_clues, &col_clues, &marks, &ans, false);
    }

    fn check_answer_conflict(
        row_clues: &[Option<i32>],
        col_clues: &[Option<i32>],
        marks: &[Vec<bool>],
        ans: &[Vec<bool>],
        expect_solvable: bool,
    ) {
        let mut solver = Solver::new();
        let shaded = add_constraints(&mut solver, row_clues, col_clues, marks);
        let (h, w) = (row_clues.len(), col_clues.len());
        for y in 0..h {
            for x in 0..w {
                if ans[y][x] {
                    solver.add_expr(shaded.at((y, x)));
                } else {
                    solver.add_expr(!shaded.at((y, x)));
                }
            }
        }
        assert_eq!(
            solver.solve().is_some(),
            expect_solvable,
            "answer satisfiability mismatch"
        );
    }
}

#[cfg(test)]
mod unique_solution_test {
    use super::*;

    #[test]
    fn test_batten_unique_solution() {
        // The 5x5 debug puzzle from pzprjs test/script/batten.js has a
        // unique solution equal to the known-valid answer.
        let problem = tests::test_problem();
        let ans = solve_batten(&problem).unwrap();
        assert!(!ans.1, "the puzzle must have a unique solution");
        let known = tests::known_answer();
        for y in 0..5 {
            for x in 0..5 {
                assert_eq!(
                    ans.0[y][x],
                    Some(known[y][x]),
                    "cell ({y}, {x}) must match the known answer"
                );
            }
        }
    }
}
