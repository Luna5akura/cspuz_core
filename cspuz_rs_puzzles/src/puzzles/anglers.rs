use crate::util;
use cspuz_rs::graph;
use cspuz_rs::solver::{count_true, Solver};

/// Anglers (Fishing): draw lines connecting each numbered person (on a cell
/// or a grid point) to a distinct fish.
///
/// Cell values (problem data):
///   -3 = shaded obstacle, -2 = ? (unknown number), -1 = empty cell,
///    0 = fish, >0 = numbered person on a cell.
/// Grid-point values: -1 = empty, -2 = ?, >0 = numbered person on a point.
///
/// Lines live on the grid-point lattice (including the outer frame, so that
/// persons on the outside can enter the board).  Lines cannot branch or
/// cross, and every unshaded cell must be covered: a numbered cell or a fish
/// is an endpoint (one incident lattice edge), ordinary cells have two
/// incident edges, obstacles none.  A number equals the number of cells on
/// its line: for a cell person the person cell itself counts (n+1 cells
/// including the fish), for a grid-point person only cells count (n cells
/// including the fish).
pub type Problem = (Vec<Vec<Option<i32>>>, Vec<Vec<Option<i32>>>);

fn infer_shape(problem: &Problem) -> (usize, usize) {
    util::infer_shape(&problem.0)
}

pub fn solve_anglers(
    problem: &Problem,
) -> Option<(Vec<Vec<Option<bool>>>, Vec<Vec<Option<bool>>>, bool)> {
    let (h, w) = infer_shape(problem);
    if h == 0 || w == 0 {
        return None;
    }
    let (cells, excs) = (&problem.0, &problem.1);
    if excs.len() != h + 1 || excs.iter().any(|row| row.len() != w + 1) {
        return None;
    }

    // Collect persons: (kind, y, x, n) with kind 0 = cell, 1 = grid point.
    let mut persons = vec![];
    for y in 0..h {
        for x in 0..w {
            if let Some(v) = cells[y][x] {
                if v > 0 || v == -2 {
                    persons.push((0, y, x, v));
                }
            }
        }
    }
    for y in 0..=h {
        for x in 0..=w {
            if let Some(v) = excs[y][x] {
                if v > 0 || v == -2 {
                    persons.push((1, y, x, v));
                }
            }
        }
    }
    let n_persons = persons.len();
    if n_persons == 0 {
        return None;
    }

    let mut solver = Solver::new();
    // Lattice edges of the grid-point graph ((h+1) x (w+1) vertices).
    let h_edge = &solver.bool_var_2d((h + 1, w)); // between (y,x) and (y,x+1)
    let v_edge = &solver.bool_var_2d((h, w + 1)); // between (y,x) and (y+1,x)
    solver.add_answer_key_bool(h_edge);
    solver.add_answer_key_bool(v_edge);

    // Four lattice edges around cell (y, x): top, bottom, left, right.
    let cell_deg = |y: usize, x: usize| -> Vec<_> {
        vec![
            h_edge.at((y, x)),
            h_edge.at((y + 1, x)),
            v_edge.at((y, x)),
            v_edge.at((y, x + 1)),
        ]
    };

    for y in 0..h {
        for x in 0..w {
            let v = cells[y][x];
            let deg = cell_deg(y, x);
            match v {
                Some(-3) => {
                    for d in deg {
                        solver.add_expr(!d);
                    }
                }
                Some(0) | Some(-2) => {
                    solver.add_expr(count_true(deg).eq(1));
                }
                Some(n) if n > 0 => {
                    solver.add_expr(count_true(deg).eq(1));
                }
                _ => {
                    solver.add_expr(count_true(deg).eq(2));
                }
            }
        }
    }

    // Lattice-point degrees: at most 2 (no branching), numbered points
    // exactly 1.
    for y in 0..=h {
        for x in 0..=w {
            let mut deg = vec![];
            if y > 0 {
                deg.push(v_edge.at((y - 1, x)));
            }
            if y < h {
                deg.push(v_edge.at((y, x)));
            }
            if x > 0 {
                deg.push(h_edge.at((y, x - 1)));
            }
            if x < w {
                deg.push(h_edge.at((y, x)));
            }
            let v = excs[y][x];
            if v.is_some_and(|n| n > 0 || n == -2) {
                solver.add_expr(count_true(deg).eq(1));
            } else {
                // Lines may turn at a lattice point (degree 2) or start/end
                // there (degree 1, only next to a fish), but must not branch
                // (degree 3+) or cross (degree 4).
                solver.add_expr(count_true(deg.clone()).le(2));
                // A degree-1 lattice point must be a fish endpoint: the
                // point is a corner of a fish cell and the line ends on one
                // of the two incident edges of that corner.
                let mut fish_edges = vec![];
                for (dy, dx) in [(0usize, 0usize), (0, 1), (1, 0), (1, 1)] {
                    if y >= dy && x >= dx {
                        let (fy, fx) = (y - dy, x - dx);
                        if fy < h && fx < w && cells[fy][fx] == Some(0) {
                            match (dy, dx) {
                                (0, 0) => {
                                    fish_edges.push(h_edge.at((fy, fx)).expr());
                                    fish_edges.push(v_edge.at((fy, fx)).expr());
                                }
                                (0, 1) => {
                                    fish_edges.push(h_edge.at((fy, fx)).expr());
                                    fish_edges.push(v_edge.at((fy, fx + 1)).expr());
                                }
                                (1, 0) => {
                                    fish_edges.push(h_edge.at((fy + 1, fx)).expr());
                                    fish_edges.push(v_edge.at((fy, fx)).expr());
                                }
                                _ => {
                                    fish_edges.push(h_edge.at((fy + 1, fx)).expr());
                                    fish_edges.push(v_edge.at((fy, fx + 1)).expr());
                                }
                            }
                        }
                    }
                }
                let is_fish_point = if fish_edges.is_empty() {
                    cspuz_rs::solver::FALSE
                } else {
                    let mut e = fish_edges[0].clone();
                    for k in 1..fish_edges.len() {
                        e = e | fish_edges[k].clone();
                    }
                    e
                };
                solver.add_expr(count_true(deg).eq(1).imp(is_fish_point));
            }
        }
    }

    // Group id per cell: 0 = not on a line (obstacle), i = person i's line.
    let group = &solver.int_var_2d((h, w), 0, n_persons as i32);
    for y in 0..h {
        for x in 0..w {
            let deg = cell_deg(y, x);
            let on_line = {
                let mut e = deg[0].expr();
                for k in 1..deg.len() {
                    e = e | deg[k].expr();
                }
                e
            };
            if cells[y][x] == Some(-3) {
                solver.add_expr(group.at((y, x)).eq(0));
            } else {
                let g = group.at((y, x));
                solver.add_expr(on_line.iff(g.ne(0)));
                // TEMP: solver.add_expr(g.ne(0).imp(on_line));
            }
        }
    }

    // Each person owns a distinct group.
    for (i, &(kind, y, x, n)) in persons.iter().enumerate() {
        let gi = (i + 1) as i32;
        if kind == 0 {
            solver.add_expr(group.at((y, x)).eq(gi));
        } else {
            // Grid-point person: exactly one incident edge is used, and the
            // adjacent cell belongs to this person's group.
            let mut adj = vec![];
            let mut adj_cells = vec![];
            if y > 0 {
                adj.push(v_edge.at((y - 1, x)));
                adj_cells.push((y - 1, x));
            }
            if y < h {
                adj.push(v_edge.at((y, x)));
                adj_cells.push((y, x));
            }
            if x > 0 {
                adj.push(h_edge.at((y, x - 1)));
                adj_cells.push((y, x - 1));
            }
            if x < w {
                adj.push(h_edge.at((y, x)));
                adj_cells.push((y, x));
            }
            for k in 0..adj.len() {
                solver.add_expr(adj[k].imp(group.at(adj_cells[k]).eq(gi)));
            }
        }

        let expected = if kind == 0 { n + 1 } else { n };
        if expected > 0 {
            solver.add_expr(group.eq(gi).count_true().eq(expected));
        }
    }

    // Every group contains exactly one fish.
    for i in 0..n_persons {
        let gi = (i + 1) as i32;
        let fish_in_group: Vec<_> = (0..h * w)
            .map(|idx| {
                let (y, x) = (idx / w, idx % w);
                group.at((y, x)).eq(gi) & (cells[y][x] == Some(0))
            })
            .collect();
        solver.add_expr(count_true(fish_in_group).eq(1));
    }

    // Connectivity within each group along used line edges.  The graph is
    // the 4-neighbor cell graph; edge indices follow infer_graph's order:
    // for each cell in row-major order, the horizontal adjacency edge first
    // (the vertical lattice edge v_edge(y, x+1)) and the vertical adjacency
    // edge second (the horizontal lattice edge h_edge(y+1, x)).
    let g = graph::infer_graph_from_2d_array((h, w));
    let mut edge_used = vec![];
    for y in 0..h {
        for x in 0..w {
            if x < w - 1 {
                edge_used.push(v_edge.at((y, x + 1)).expr());
            }
            if y < h - 1 {
                edge_used.push(h_edge.at((y + 1, x)).expr());
            }
        }
    }
    for i in 0..n_persons {
        let gi = (i + 1) as i32;
        let mut vertices = vec![];
        for idx in 0..h * w {
            let (y, x) = (idx / w, idx % w);
            vertices.push(group.at((y, x)).eq(gi));
        }

        graph::active_vertices_connected(&mut solver, vertices, &g);
    }

    // Enumerate answers.  If the enumeration is cut off (too many answers),
    // no single edge is guaranteed to hold, so return an empty partial
    // answer marked as non-unique (the UI then shows only the problem data).
    const MAX_ENUM: usize = 5000;
    let answers: Vec<_> = solver.answer_iter().take(MAX_ENUM).collect();
    if answers.is_empty() {
        return None;
    }
    let truncated = answers.len() >= MAX_ENUM;
    if truncated {
        return Some((
            vec![vec![None; w]; h + 1],
            vec![vec![None; w + 1]; h],
            true,
        ));
    }
    let mut h_out = vec![vec![None; w]; h + 1];
    let mut v_out = vec![vec![None; w + 1]; h];
    for y in 0..h + 1 {
        for x in 0..w {
            let v = answers[0].get_unwrap(&h_edge.at((y, x)));
            if answers.iter().all(|a| a.get_unwrap(&h_edge.at((y, x))) == v) {
                h_out[y][x] = Some(v);
            }
        }
    }
    for y in 0..h {
        for x in 0..w + 1 {
            let v = answers[0].get_unwrap(&v_edge.at((y, x)));
            if answers.iter().all(|a| a.get_unwrap(&v_edge.at((y, x))) == v) {
                v_out[y][x] = Some(v);
            }
        }
    }
    Some((h_out, v_out, truncated))
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

/// Encode one value in pzpr's anglers format:
///   -3 -> 0, 0 -> 1, n>0 -> n+1 (hex), -2 -> "."
fn encode_value(v: Option<i32>) -> Option<String> {
    match v {
        None => None,
        Some(-3) => Some("0".to_string()),
        Some(0) => Some("1".to_string()),
        Some(n) if n > 0 && n < 15 => Some(format!("{:x}", n + 1)),
        Some(-2) => Some(".".to_string()),
        _ => None,
    }
}

fn encode_payload(problem: &Problem) -> Option<String> {
    let (cells, excs) = problem;
    let mut payload = String::new();
    let mut skipped = 0usize;
    for row in cells {
        for &v in row {
            match encode_value(v) {
                None => skipped += 1,
                Some(sv) => {
                    flush_skips(&mut payload, &mut skipped);
                    payload.push_str(&sv);
                }
            }
        }
    }
    for row in excs {
        for &v in row {
            match encode_value(v) {
                None => skipped += 1,
                Some(sv) => {
                    flush_skips(&mut payload, &mut skipped);
                    payload.push_str(&sv);
                }
            }
        }
    }
    flush_skips(&mut payload, &mut skipped);
    Some(payload)
}

fn decode_payload(payload: &[u8], h: usize, w: usize) -> Option<Problem> {
    let n_cells = h * w;
    let n_excs = (h + 1) * (w + 1);
    let total = n_cells + n_excs;
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
        let (val, consumed) = match code {
            b'.' => (Some(-2), 1),
            b'0'..=b'9' | b'a'..=b'e' => {
                let d = hex_digit(code)?;
                let v = if d == 0 { -3 } else { d - 1 };
                (Some(v), 1)
            }
            _ => return None,
        };
        if index >= total {
            return None;
        }
        values[index] = val;
        index += 1;
        pos += consumed;
    }
    let cells = values[..n_cells]
        .chunks(w)
        .map(|row| row.to_vec())
        .collect::<Vec<_>>();
    let excs = values[n_cells..]
        .chunks(w + 1)
        .map(|row| row.to_vec())
        .collect::<Vec<_>>();
    Some((cells, excs))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = infer_shape(problem);
    if h == 0 || w == 0 {
        return None;
    }
    let payload = encode_payload(problem)?;
    Some(format!("https://puzz.link/p?anglers/{w}/{h}/{payload}"))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let payload = url.split('?').nth(1)?;
    let mut parts = payload.split('/');
    if parts.next()? != "anglers" {
        return None;
    }
    let w = parts.next()?.parse().ok()?;
    let h = parts.next()?.parse().ok()?;
    decode_payload(parts.next().unwrap_or("").as_bytes(), h, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_problem() -> Problem {
        // 1x3: a person numbered 3 on the top-left grid point, one fish in
        // the rightmost cell.  The line covers all three cells.
        let cells = vec![vec![None, None, Some(0)]];
        let excs = vec![
            vec![Some(3), None, None, None],
            vec![None, None, None, None],
        ];
        (cells, excs)
    }

    #[test]
    fn test_anglers_serializer() {
        let problem = simple_problem();
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(url, "https://puzz.link/p?anglers/3/1/h14m");
        let decoded = deserialize_problem(&url).unwrap();
        assert_eq!(decoded, problem);
    }

    #[test]
    fn test_anglers_problem() {
        let problem = simple_problem();
        let ans = solve_anglers(&problem).unwrap();
        // The problem has two valid lines (entering the board from the
        // top-left point either downward or rightward); the solver returns
        // their intersection.
        assert_eq!(ans.0[1][1], Some(true)); // shared horizontal edge
        assert_eq!(ans.0[0][1], Some(false));
        assert_eq!(ans.1[0][3], Some(false)); // right frame edge unused
        assert_eq!(ans.0[0][0], None); // differs between the two answers
        assert_eq!(ans.1[0][0], None);
    }
}




