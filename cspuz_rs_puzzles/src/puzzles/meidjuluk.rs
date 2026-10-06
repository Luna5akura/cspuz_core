use crate::util;
use cspuz_rs::solver::{count_true, Config, GraphDivisionMode, Solver};

/// Meidjuluk: draw lines over the dotted lines (edges between non-shaded
/// cells) to divide the board into blocks.
///
/// Rules:
/// 1. Shaded cells are not part of any block.
/// 2. Exactly one block must exist of each size given in the bank; no other
///    blocks exist. (Each bank entry is one required block, so a size listed
///    twice requires two blocks of that size.)
/// 3. A number must be a divisor of the size of the block it is in.
/// 4. A block cannot contain two identical numbers.
///
/// Cell values: Some(0) = shaded (drawn black, not part of any block),
/// None = white empty cell, Some(n>0) = number.
pub type Problem = (Vec<Vec<Option<i32>>>, Vec<i32>);

fn infer_shape(problem: &Problem) -> (usize, usize) {
    util::infer_shape(&problem.0)
}

/// Creates a solver configured for the graph division propagator.
///
/// The Rust graph division propagator is used because the C++ one may not be
/// available in the wasm backend (fillomino does the same).
fn new_solver<'a>() -> Solver<'a> {
    let mut config = Config::default();
    config.graph_division_mode = GraphDivisionMode::Rust;
    Solver::with_config(config)
}

pub fn solve_meidjuluk(
    problem: &Problem,
) -> Option<(Vec<Vec<Option<bool>>>, Vec<Vec<Option<bool>>>, bool)> {
    let (h, w) = infer_shape(problem);
    if h == 0 || w == 0 {
        return None;
    }
    let (cells, bank) = (&problem.0, &problem.1);
    if bank.is_empty() {
        return None;
    }

    // Build the constraints twice: first to extract irrefutable facts
    // (lines that must hold in every answer), then to enumerate answers.
    let facts = {
        let mut s = new_solver();
        let (h_edge, v_edge) = add_constraints(&mut s, cells, bank)?;
        s.add_answer_key_bool(&h_edge);
        s.add_answer_key_bool(&v_edge);
        match s.irrefutable_facts() {
            Some(f) => (f.get(&h_edge), f.get(&v_edge)),
            None => (vec![], vec![]),
        }
    };
    let mut solver = new_solver();
    let (h_edge, v_edge) = add_constraints(&mut solver, cells, bank)?;
    solver.add_answer_key_bool(&h_edge);
    solver.add_answer_key_bool(&v_edge);
    // Enumerate and merge.
    const MAX_ENUM: usize = 5000;
    let answers: Vec<_> = solver.answer_iter().take(MAX_ENUM).collect();
    if answers.is_empty() {
        return None;
    }
    let truncated = answers.len() >= MAX_ENUM;
    if truncated {
        // Return the irrefutable facts (lines that must hold in every
        // answer) instead of an empty result.
        return Some((facts.0, facts.1, true));
    }
    let mut h_out = vec![vec![None; w - 1]; h];
    let mut v_out = vec![vec![None; w]; h - 1];
    for y in 0..h {
        for x in 0..w - 1 {
            let v = answers[0].get_unwrap(&h_edge.at((y, x)));
            if answers.iter().all(|a| a.get_unwrap(&h_edge.at((y, x))) == v) {
                h_out[y][x] = Some(v);
            }
        }
    }
    for y in 0..h - 1 {
        for x in 0..w {
            let v = answers[0].get_unwrap(&v_edge.at((y, x)));
            if answers.iter().all(|a| a.get_unwrap(&v_edge.at((y, x))) == v) {
                v_out[y][x] = Some(v);
            }
        }
    }
    Some((h_out, v_out, truncated))
}

fn add_constraints(
    solver: &mut Solver,
    cells: &[Vec<Option<i32>>],
    bank: &[i32],
) -> Option<(
    cspuz_rs::solver::BoolVarArray2D,
    cspuz_rs::solver::BoolVarArray2D,
)> {
    let (h, w) = (cells.len(), cells[0].len());
    let is_shaded = |y: usize, x: usize| cells[y][x] == Some(0);

    // Answer keys: lines over the dotted lines, i.e. edges between
    // non-shaded cells. Edges adjacent to a shaded cell cannot be drawn on.
    let h_edge = solver.bool_var_2d((h, w - 1));
    let v_edge = solver.bool_var_2d((h - 1, w));
    for y in 0..h {
        for x in 0..w - 1 {
            if is_shaded(y, x) || is_shaded(y, x + 1) {
                solver.add_expr(!h_edge.at((y, x)));
            }
        }
    }
    for y in 0..h - 1 {
        for x in 0..w {
            if is_shaded(y, x) || is_shaded(y + 1, x) {
                solver.add_expr(!v_edge.at((y, x)));
            }
        }
    }

    // Distinct bank values with their multiplicities. pzprjs treats every
    // bank entry as one required block, so a size listed m times needs
    // exactly m blocks of that size.
    let mut bank_map = std::collections::BTreeMap::new();
    for &v in bank {
        *bank_map.entry(v).or_insert(0) += 1;
    }
    let distinct: Vec<i32> = bank_map.keys().cloned().collect();

    // Vertices of the region graph are the non-shaded cells. Shaded cells
    // are walls and are not part of any block.
    let mut cell_id = vec![vec![None; w]; h];
    let mut vertex_cells = vec![];
    for y in 0..h {
        for x in 0..w {
            if !is_shaded(y, x) {
                cell_id[y][x] = Some(vertex_cells.len());
                vertex_cells.push((y, x));
            }
        }
    }
    let n_vertices = vertex_cells.len();

    // Per-vertex block-size variable. A number must divide the size of its
    // block (rule 3), so restrict the domain to the matching bank values.
    let mut sizes = Vec::with_capacity(n_vertices);
    for &(y, x) in &vertex_cells {
        let domain: Vec<i32> = match cells[y][x] {
            Some(n) if n > 0 => distinct
                .iter()
                .cloned()
                .filter(|&v| v % n == 0)
                .collect(),
            _ => distinct.clone(),
        };
        if domain.is_empty() {
            return None;
        }
        sizes.push(solver.int_var_from_domain(domain));
    }

    // Edges between adjacent non-shaded cells; TRUE = dividing line.
    let mut edges = vec![];
    let mut edge_vars: Vec<cspuz_rs::solver::BoolExpr> = vec![];
    for y in 0..h {
        for x in 0..w {
            if x < w - 1 {
                if let (Some(a), Some(b)) = (cell_id[y][x], cell_id[y][x + 1]) {
                    edges.push((a, b));
                    edge_vars.push(h_edge.at((y, x)).expr());
                }
            }
            if y < h - 1 {
                if let (Some(a), Some(b)) = (cell_id[y][x], cell_id[y + 1][x]) {
                    edges.push((a, b));
                    edge_vars.push(v_edge.at((y, x)).expr());
                }
            }
        }
    }

    // Blocks are exactly the connected components of non-shaded cells via
    // edges without a line, and the size variable of every cell in a block
    // must equal the number of cells of that block.
    let sizes_opt: Vec<Option<cspuz_rs::solver::IntExpr>> =
        sizes.iter().map(|s| Some(s.expr())).collect();
    solver.add_graph_division(&sizes_opt, &edges, &edge_vars);

    // Rule 2: the bank value v appears m times, so exactly v*m cells have
    // block size v. Combined with the graph division (each block of size v
    // has v cells) this means exactly m blocks of size v exist.
    for &v in &distinct {
        let m = bank_map[&v];
        let exprs: Vec<cspuz_rs::solver::BoolExpr> =
            sizes.iter().map(|s| s.eq(v)).collect();
        solver.add_expr(count_true(exprs).eq(v * m));
    }

    // Rule 4: cells with the same number must lie in different blocks.
    // A block is a connected component via line-free edges, so cells of one
    // number class must be pairwise disconnected in that graph. Encode this
    // with rank variables: every cell of the class gets a distinct fixed
    // rank, and each line-free edge forces the two cells to share a rank.
    let mut num_positions: std::collections::HashMap<i32, Vec<usize>> =
        std::collections::HashMap::new();
    for (id, &(y, x)) in vertex_cells.iter().enumerate() {
        if let Some(n) = cells[y][x] {
            if n > 0 {
                num_positions.entry(n).or_default().push(id);
            }
        }
    }
    for positions in num_positions.values() {
        let k = positions.len();
        if k < 2 {
            continue;
        }
        let rank: Vec<cspuz_rs::solver::IntVar> = (0..n_vertices)
            .map(|_| solver.int_var(0, (k - 1) as i32))
            .collect();
        for (i, &p) in positions.iter().enumerate() {
            solver.add_expr(rank[p].eq(i as i32));
        }
        for i in 0..edges.len() {
            let (a, b) = edges[i];
            solver.add_expr((!edge_vars[i].clone()).imp(rank[a].clone().eq(rank[b].clone())));
        }
    }

    Some((h_edge, v_edge))
}

fn hex_digit(c: u8) -> Option<i32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i32),
        b'a'..=b'f' => Some((c - b'a' + 10) as i32),
        b'A'..=b'F' => Some((c - b'A' + 10) as i32),
        _ => None,
    }
}

fn flush_skips(payload: &mut String, skipped: &mut usize) {
    while *skipped > 0 {
        let chunk = (*skipped).min(20);
        payload.push((b'g' + chunk as u8 - 1) as char);
        *skipped -= chunk;
    }
}

fn encode_cells(cells: &[Vec<Option<i32>>]) -> Option<String> {
    let mut payload = String::new();
    let mut skipped = 0usize;
    for row in cells {
        for &c in row {
            match c {
                None => skipped += 1,
                Some(0) => {
                    flush_skips(&mut payload, &mut skipped);
                    payload.push('0');
                }
                Some(n) if n > 0 && n < 16 => {
                    flush_skips(&mut payload, &mut skipped);
                    payload.push(char::from_digit(n as u32, 16)?);
                }
                _ => return None,
            }
        }
    }
    flush_skips(&mut payload, &mut skipped);
    Some(payload)
}

fn decode_cells(payload: &[u8], h: usize, w: usize) -> Option<Vec<Vec<Option<i32>>>> {
    let total = h * w;
    let mut values = vec![None; total];
    let mut index = 0usize;
    let mut pos = 0usize;
    while index < total && pos < payload.len() {
        let code = payload[pos];
        if (b'g'..=b'z').contains(&code) {
            index += (code - b'g' + 1) as usize;
            pos += 1;
            continue;
        }
        let val = match code {
            b'0'..=b'9' | b'a'..=b'f' => Some(hex_digit(code)?),
            _ => return None,
        };
        if index >= total {
            return None;
        }
        values[index] = val.map(|v| if v == 0 { 0 } else { v });
        index += 1;
        pos += 1;
    }
    Some(values.chunks(w).map(|row| row.to_vec()).collect())
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = infer_shape(problem);
    if h == 0 || w == 0 {
        return None;
    }
    let payload = encode_cells(&problem.0)?;
    let bank = &problem.1;
    let mut s = format!("https://puzz.link/p?meidjuluk/{w}/{h}/{payload}");
    // Bank encoding matches pzprjs (snakeegg.js encodePieceBank):
    // - empty -> "//z"
    // - 1..=9 -> "//i"
    // - any other 1..=n -> "//n"
    // - anything else -> "/count/v1/v2/..."
    let is_consecutive = bank
        .iter()
        .enumerate()
        .all(|(i, &v)| v == (i + 1) as i32);
    if bank.is_empty() {
        s.push_str("//z");
    } else if is_consecutive && bank.len() <= 999 {
        if bank.len() == 9 {
            s.push_str("//i");
        } else {
            s.push_str(&format!("//{}", bank.len()));
        }
    } else {
        s.push('/');
        s.push_str(&bank.len().to_string());
        for &v in bank {
            s.push('/');
            s.push_str(&v.to_string());
        }
    }
    Some(s)
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let payload = url.split('?').nth(1)?;
    let mut parts = payload.split('/');
    if parts.next()? != "meidjuluk" {
        return None;
    }
    let w = parts.next()?.parse().ok()?;
    let h = parts.next()?.parse().ok()?;
    let cell_payload = parts.next()?;
    let cells = decode_cells(cell_payload.as_bytes(), h, w)?;
    // The bank is a preset ("//i" = 1..=9, "//z" = empty, "//n" = 1..=n) or
    // an explicit list "/count/v1/v2/...".
    let bank_next = parts.next()?;
    let bank = if bank_next.is_empty() {
        match parts.next()? {
            "i" => (1..=9).collect(),
            "z" => vec![],
            s => {
                let n: i32 = s.parse().ok()?;
                if n <= 0 {
                    vec![]
                } else {
                    (1..=n).collect()
                }
            }
        }
    } else {
        let count: usize = bank_next.parse().ok()?;
        let mut bank = vec![];
        for _ in 0..count {
            bank.push(parts.next()?.parse().ok()?);
        }
        bank
    };
    Some((cells, bank))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_problem() -> Problem {
        // 4x4 without shaded cells: the line divides a 4-cell block
        // containing the 2 from the remaining 12-cell block containing 1.
        let cells = vec![
            vec![Some(2), None, None, None],
            vec![None, None, None, None],
            vec![None, None, None, None],
            vec![None, None, None, Some(1)],
        ];
        (cells, vec![4, 12])
    }

    /// 2x2 with a unique solution: the "1" must be a block of size 1, and
    /// the "3" joins the other two cells to form the block of size 3.
    fn unique_problem() -> Problem {
        let cells = vec![vec![Some(3), None], vec![Some(1), None]];
        (cells, vec![1, 3])
    }


    #[test]
    fn test_meidjuluk_serializer() {
        let problem = simple_problem();
        let url = serialize_problem(&problem).unwrap();
        let decoded = deserialize_problem(&url).unwrap();
        assert_eq!(decoded, problem);
    }

    #[test]
    fn test_meidjuluk_serializer_range_bank() {
        let cells = vec![
            vec![Some(3), None, None, Some(1), Some(0), None],
            vec![Some(5), Some(0), Some(2), Some(4), Some(0), None],
            vec![Some(1), None, None, Some(1), Some(2), None],
            vec![None, Some(1), Some(2), None, None, None],
        ];
        let problem = (cells, (1..=6).collect::<Vec<_>>());
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(
            url,
            "https://puzz.link/p?meidjuluk/6/4/3h10g50240g1h12h12i//6"
        );
        assert_eq!(deserialize_problem(&url).unwrap(), problem);
    }

    #[test]
    fn test_meidjuluk_pzpr_url() {
        let url = "https://puzz.link/p?meidjuluk/4/4/20g1g0i0g2g0h/2/4/8";
        let decoded = deserialize_problem(url).unwrap();
        let expected = (
            vec![
                vec![Some(2), Some(0), None, Some(1)],
                vec![None, Some(0), None, None],
                vec![None, Some(0), None, Some(2)],
                vec![None, Some(0), None, None],
            ],
            vec![4, 8],
        );
        assert_eq!(decoded, expected);
        assert_eq!(serialize_problem(&decoded).unwrap(), url);
    }

    #[test]
    fn test_meidjuluk_solver_runs() {
        let problem = simple_problem();
        let ans = solve_meidjuluk(&problem);
        assert!(ans.is_some());
    }

    #[test]
    fn test_meidjuluk_unique_solution() {
        // The 4x6 debug board from pzprjs test/script/meidjuluk.js has a
        // unique solution, and the test data contains the known-good answer
        // (the failcheck entry with `null` failcode).
        let url = "https://puzz.link/p?meidjuluk/6/4/3h10g50240g1h12h12i/6/1/2/3/4/5/6";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_meidjuluk(&problem).unwrap();
        let (h_out, v_out, truncated) = ans;
        assert!(!truncated, "the puzzle must have a unique solution");
        let h_rows = [
            [0, 0, 1, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 1, 1, 0],
            [1, 0, 1, 1, 0],
        ];
        let v_rows = [
            [1, 0, 1, 1, 0, 0],
            [0, 0, 1, 0, 0, 0],
            [0, 1, 1, 0, 0, 0],
        ];
        for y in 0..4 {
            for x in 0..5 {
                assert_eq!(
                    h_out[y][x],
                    Some(h_rows[y][x] == 1),
                    "h line at ({y}, {x}) must match the known answer"
                );
            }
        }
        for y in 0..3 {
            for x in 0..6 {
                assert_eq!(
                    v_out[y][x],
                    Some(v_rows[y][x] == 1),
                    "v line at ({y}, {x}) must match the known answer"
                );
            }
        }
    }

    #[test]
    fn test_meidjuluk_rejects_merged_blocks() {
        // Without any drawn line the two blocks would merge into one block
        // of size 16, which is not in the bank. The old model missed this
        // separation constraint and accepted the line-free answer.
        let problem = simple_problem();
        let (cells, bank) = &problem;
        let (h, w) = (4usize, 4usize);
        let mut solver = new_solver();
        let (h_edge, v_edge) = add_constraints(&mut solver, cells, bank).unwrap();
        solver.add_answer_key_bool(&h_edge);
        solver.add_answer_key_bool(&v_edge);
        for y in 0..h {
            for x in 0..w - 1 {
                solver.add_expr(!h_edge.at((y, x)));
            }
        }
        for y in 0..h - 1 {
            for x in 0..w {
                solver.add_expr(!v_edge.at((y, x)));
            }
        }
        assert_eq!(solver.solve().is_none(), true, "line-free board must be rejected");
    }
}

#[cfg(test)]
mod preset_test {
    use super::*;

    #[test]
    fn test_preset_bank_url() {
        let url = "https://puzz.link/p?meidjuluk/7/7/1h3h9g1i7i105h2g010g3h401i6g3g1g8h2h1//i";
        let decoded = deserialize_problem(url).unwrap();
        assert_eq!(decoded.1, (1..=9).collect::<Vec<_>>());
        assert_eq!(serialize_problem(&decoded).unwrap(), url);
    }

    #[test]
    fn test_empty_bank_url() {
        let url = "https://puzz.link/p?meidjuluk/2/2/3g1g//z";
        let decoded = deserialize_problem(url).unwrap();
        assert_eq!(decoded.1, Vec::<i32>::new());
        assert_eq!(serialize_problem(&decoded).unwrap(), url);
    }
}

#[cfg(test)]
mod user_sol_test {
    use super::*;

    /// Validates an answer (line configuration) against the pzprjs check
    /// semantics directly: blocks are connected components of non-shaded
    /// cells via line-free edges, shaded cells are walls.
    pub(super) fn check_answer(
        cells: &[Vec<Option<i32>>],
        bank: &[i32],
        h_rows: &[Vec<bool>],
        v_rows: &[Vec<bool>],
    ) -> bool {
        let (h, w) = (cells.len(), cells[0].len());
        let is_shaded = |y: usize, x: usize| cells[y][x] == Some(0);
        // No line on a shaded-adjacent edge.
        for y in 0..h {
            for x in 0..w - 1 {
                if h_rows[y][x] && (is_shaded(y, x) || is_shaded(y, x + 1)) {
                    return false;
                }
            }
        }
        for y in 0..h - 1 {
            for x in 0..w {
                if v_rows[y][x] && (is_shaded(y, x) || is_shaded(y + 1, x)) {
                    return false;
                }
            }
        }
        // Compute blocks.
        let mut seen = vec![vec![false; w]; h];
        let mut sizes = vec![];
        for y in 0..h {
            for x in 0..w {
                if seen[y][x] || is_shaded(y, x) {
                    continue;
                }
                let mut cnt = 0;
                let mut nums = vec![];
                let mut stack = vec![(y, x)];
                seen[y][x] = true;
                while let Some((cy, cx)) = stack.pop() {
                    cnt += 1;
                    if let Some(n) = cells[cy][cx] {
                        if n > 0 {
                            nums.push(n);
                        }
                    }
                    for (dy, dx) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
                        let ny = cy as i32 + dy;
                        let nx = cx as i32 + dx;
                        if ny < 0 || nx < 0 || ny >= h as i32 || nx >= w as i32 {
                            continue;
                        }
                        let (ny, nx) = (ny as usize, nx as usize);
                        if seen[ny][nx] || is_shaded(ny, nx) {
                            continue;
                        }
                        let blocked = if dy == 0 {
                            let x0 = cx.min(nx);
                            h_rows[cy][x0]
                        } else {
                            let y0 = cy.min(ny);
                            v_rows[y0][cx]
                        };
                        if !blocked {
                            seen[ny][nx] = true;
                            stack.push((ny, nx));
                        }
                    }
                }
                sizes.push((cnt, nums));
            }
        }
        // Block sizes match the bank (multiplicity matters).
        let mut bank_sorted: Vec<i32> = bank.to_vec();
        bank_sorted.sort_unstable();
        let mut size_sorted: Vec<i32> = sizes.iter().map(|s| s.0 as i32).collect();
        size_sorted.sort_unstable();
        if size_sorted != bank_sorted {
            return false;
        }
        // Rules 3 and 4.
        for (cnt, nums) in &sizes {
            for &n in nums {
                if cnt % n as usize != 0 {
                    return false;
                }
            }
            let mut sorted = nums.clone();
            sorted.sort_unstable();
            for i in 1..sorted.len() {
                if sorted[i] == sorted[i - 1] {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn user_puzzle_solvable() {
        let url = "https://puzz.link/p?meidjuluk/7/7/1h3h9g1i7i105h2g010g3h401i6g3g1g8h2h1//i";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_meidjuluk(&problem);
        assert!(ans.is_some(), "the user's puzzle must be solvable");
    }

    #[test]
    fn user_solution_satisfies_constraints() {
        let url = "https://puzz.link/p?meidjuluk/7/7/1h3h9g1i7i105h2g010g3h401i6g3g1g8h2h1//i";
        let problem = deserialize_problem(url).unwrap();
        let (cells, bank) = &problem;
        let (h, w) = (7usize, 7usize);
        // 用户的答案线 (水平 7x6, 垂直 6x7)
        let h_rows: Vec<Vec<bool>> = vec![
            vec![true, false, false, true, false, true],
            vec![true, true, false, false, false, true],
            vec![true, true, false, false, false, true],
            vec![true, false, false, false, false, true],
            vec![false, false, false, false, true, true],
            vec![true, false, false, false, true, true],
            vec![true, true, false, true, false, false],
        ];
        let v_rows: Vec<Vec<bool>> = vec![
            vec![false, true, true, true, false, false, false],
            vec![false, false, false, false, true, true, false],
            vec![false, false, false, false, false, false, false],
            vec![true, false, false, false, false, false, false],
            vec![false, true, true, false, false, false, false],
            vec![false, false, true, true, true, true, false],
        ];
        // The user's answer must be a valid pzprjs solution.
        assert!(
            check_answer(cells, bank, &h_rows, &v_rows),
            "the user's answer must satisfy the pzprjs checks"
        );
        // The user's answer must also satisfy the solver's constraints.
        let mut solver = new_solver();
        let (h_edge, v_edge) = add_constraints(&mut solver, cells, bank).unwrap();
        solver.add_answer_key_bool(&h_edge);
        solver.add_answer_key_bool(&v_edge);
        for y in 0..h {
            for x in 0..w - 1 {
                if h_rows[y][x] {
                    solver.add_expr(h_edge.at((y, x)));
                } else {
                    solver.add_expr(!h_edge.at((y, x)));
                }
            }
        }
        for y in 0..h - 1 {
            for x in 0..w {
                if v_rows[y][x] {
                    solver.add_expr(v_edge.at((y, x)));
                } else {
                    solver.add_expr(!v_edge.at((y, x)));
                }
            }
        }
        assert!(
            solver.solve().is_some(),
            "the user's answer must satisfy all constraints"
        );
    }
}

#[cfg(test)]
mod answer_validity_test {
    use super::user_sol_test::check_answer;
    use super::*;

    #[test]
    fn every_answer_satisfies_pzpr_checks() {
        // The 4x6 debug board from pzprjs test/script/meidjuluk.js.
        let url = "https://puzz.link/p?meidjuluk/6/4/3h10g50240g1h12h12i/6/1/2/3/4/5/6";
        let problem = deserialize_problem(url).unwrap();
        let (cells, bank) = (&problem.0, &problem.1);
        let (h, w) = (cells.len(), cells[0].len());
        let mut solver = new_solver();
        let (h_edge, v_edge) = add_constraints(&mut solver, cells, bank).unwrap();
        solver.add_answer_key_bool(&h_edge);
        solver.add_answer_key_bool(&v_edge);
        let mut checked = 0;
        for answer in solver.answer_iter().take(200) {
            let h_rows: Vec<Vec<bool>> = (0..h)
                .map(|y| {
                    (0..w - 1)
                        .map(|x| answer.get_unwrap(&h_edge.at((y, x))))
                        .collect()
                })
                .collect();
            let v_rows: Vec<Vec<bool>> = (0..h - 1)
                .map(|y| {
                    (0..w)
                        .map(|x| answer.get_unwrap(&v_edge.at((y, x))))
                        .collect()
                })
                .collect();
            assert!(
                check_answer(cells, bank, &h_rows, &v_rows),
                "every solver answer must satisfy the pzprjs checks"
            );
            checked += 1;
        }
        assert!(checked > 0, "the puzzle must have at least one answer");
    }

    #[test]
    fn user_example_runs() {
        let url = "https://puzz.link/p?meidjuluk/7/7/11g3h941i7i105h22010g3h401i6g3g1g8h2h1//i";
        let problem = deserialize_problem(url).unwrap();
        let ans = solve_meidjuluk(&problem);
        assert!(ans.is_some(), "the user's puzzle must be solvable");
        let (h_out, v_out, _) = ans.unwrap();
        let (h, w) = (7usize, 7usize);
        // Facts must never place a line on a shaded-adjacent edge.
        let is_shaded = |y: usize, x: usize| problem.0[y][x] == Some(0);
        for y in 0..h {
            for x in 0..w - 1 {
                if h_out[y][x] == Some(true) {
                    assert!(!is_shaded(y, x) && !is_shaded(y, x + 1));
                }
            }
        }
        for y in 0..h - 1 {
            for x in 0..w {
                if v_out[y][x] == Some(true) {
                    assert!(!is_shaded(y, x) && !is_shaded(y + 1, x));
                }
            }
        }
    }
}
