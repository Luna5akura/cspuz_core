use crate::util;
use cspuz_rs::graph;
use cspuz_rs::solver::{count_true, Solver};

/// Dotchi Dotchi Loop (dotchi2): draw a single loop (no branching or
/// crossing) on a grid divided into regions.
///
/// In every region, the loop must pass through either all black circles and
/// no white circles, or all white circles and no black circles.  Every
/// region must have at least one circle passed by the loop.  The loop goes
/// straight through white circles and turns on black circles.
///
/// Problem data: circles (0 = none, 1 = white, 2 = black) plus region border
/// lines (thick borders between cells).
pub type Problem = (
    Vec<Vec<Option<i32>>>, // circles
    Vec<Vec<bool>>,        // horizontal borders (h, w-1)
    Vec<Vec<bool>>,        // vertical borders (h-1, w)
);

fn infer_shape(problem: &Problem) -> (usize, usize) {
    util::infer_shape(&problem.0)
}

pub fn solve_dotchi2(
    problem: &Problem,
) -> Option<(Vec<Vec<Option<bool>>>, Vec<Vec<Option<bool>>>, bool)> {
    let (h, w) = infer_shape(problem);
    if h == 0 || w == 0 {
        return None;
    }
    let (circles, bh, bv) = (&problem.0, &problem.1, &problem.2);
    if bh.len() != h || bh.iter().any(|r| r.len() != w - 1) {
        return None;
    }
    if bv.len() != h - 1 || bv.iter().any(|r| r.len() != w) {
        return None;
    }

    // Region ids: cells separated by thick borders belong to different
    // regions.
    let mut room = vec![vec![0usize; w]; h];
    let mut n_rooms = 0;
    for y in 0..h {
        for x in 0..w {
            if room[y][x] != 0 {
                continue;
            }
            n_rooms += 1;
            let rid = n_rooms;
            let mut stack = vec![(y, x)];
            room[y][x] = rid;
            while let Some((cy, cx)) = stack.pop() {
                for (dy, dx) in [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
                    let ny = cy as i32 + dy;
                    let nx = cx as i32 + dx;
                    if ny < 0 || nx < 0 || ny >= h as i32 || nx >= w as i32 {
                        continue;
                    }
                    let (ny, nx) = (ny as usize, nx as usize);
                    // Check the border between (cy,cx) and (ny,nx).
                    let blocked = if dy == 0 {
                        let x0 = cx.min(nx);
                        bh[cy][x0]
                    } else {
                        let y0 = cy.min(ny);
                        bv[y0][cx]
                    };
                    if blocked {
                        continue;
                    }
                    if room[ny][nx] == 0 {
                        room[ny][nx] = rid;
                        stack.push((ny, nx));
                    }
                }
            }
        }
    }

    let mut solver = Solver::new();
    let h_edge = &solver.bool_var_2d((h, w - 1));
    let v_edge = &solver.bool_var_2d((h - 1, w));
    solver.add_answer_key_bool(h_edge);
    solver.add_answer_key_bool(v_edge);

    let deg = |y: usize, x: usize| -> Vec<_> {
        let mut d = vec![];
        if y > 0 {
            d.push(v_edge.at((y - 1, x)));
        }
        if y < h - 1 {
            d.push(v_edge.at((y, x)));
        }
        if x > 0 {
            d.push(h_edge.at((y, x - 1)));
        }
        if x < w - 1 {
            d.push(h_edge.at((y, x)));
        }
        d
    };

    let cell_used = |y: usize, x: usize| {
        let d = deg(y, x);
        let mut e = d[0].expr();
        for k in 1..d.len() {
            e = e | d[k].expr();
        }
        e
    };

    for y in 0..h {
        for x in 0..w {
            let d = deg(y, x);
            let used = count_true(d.clone()).eq(2);
            match circles[y][x] {
                Some(1) => {
                    // White circle: straight when passed.  The loop may also
                    // skip the circle (then degree 0).
                    solver.add_expr(count_true(d.clone()).eq(0) | used.clone());
                    // Straight: opposite edges used together.
                    let horz = if x > 0 && x < w - 1 {
                        h_edge.at((y, x - 1)).expr() & h_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    let vert = if y > 0 && y < h - 1 {
                        v_edge.at((y - 1, x)).expr() & v_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    solver.add_expr(used.imp(horz | vert));
                }
                Some(2) => {
                    // Black circle: turn when passed.
                    solver.add_expr(count_true(d.clone()).eq(0) | used.clone());
                    let horz = if x > 0 && x < w - 1 {
                        h_edge.at((y, x - 1)).expr() & h_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    let vert = if y > 0 && y < h - 1 {
                        v_edge.at((y - 1, x)).expr() & v_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    // Turn: not straight.
                    solver.add_expr(used.imp(!(horz | vert)));
                }
                _ => {
                    // Ordinary cell: degree 0 or 2.
                    let two = count_true(d).eq(2);
                    solver.add_expr(two | count_true(deg(y, x)).eq(0));
                }
            }
        }
    }

    // Region constraints.
    for r in 1..=n_rooms {
        let mut moon_cells = vec![];
        let mut sun_cells = vec![];
        for y in 0..h {
            for x in 0..w {
                if room[y][x] != r {
                    continue;
                }
                match circles[y][x] {
                    Some(1) => sun_cells.push((y, x)),
                    Some(2) => moon_cells.push((y, x)),
                    _ => {}
                }
            }
        }
        if moon_cells.is_empty() && sun_cells.is_empty() {
            continue;
        }
        let moon_used: Vec<_> = moon_cells.iter().map(|&(y, x)| cell_used(y, x)).collect();
        let sun_used: Vec<_> = sun_cells.iter().map(|&(y, x)| cell_used(y, x)).collect();
        let all_moon = if moon_cells.is_empty() {
            cspuz_rs::solver::TRUE
        } else {
            let mut e = moon_used[0].clone();
            for k in 1..moon_used.len() {
                e = e & moon_used[k].clone();
            }
            e
        };
        let no_sun = if sun_used.is_empty() {
            cspuz_rs::solver::TRUE
        } else {
            let mut e = !sun_used[0].clone();
            for k in 1..sun_used.len() {
                e = e & !sun_used[k].clone();
            }
            e
        };
        let all_sun = if sun_cells.is_empty() {
            cspuz_rs::solver::TRUE
        } else {
            let mut e = sun_used[0].clone();
            for k in 1..sun_used.len() {
                e = e & sun_used[k].clone();
            }
            e
        };
        let no_moon = if moon_used.is_empty() {
            cspuz_rs::solver::TRUE
        } else {
            let mut e = !moon_used[0].clone();
            for k in 1..moon_used.len() {
                e = e & !moon_used[k].clone();
            }
            e
        };
        solver.add_expr((all_moon & no_sun) | (all_sun & no_moon));

        // At least one circle in the region is passed.
        let any_used = {
            let mut e = if !moon_used.is_empty() {
                moon_used[0].clone()
            } else {
                sun_used[0].clone()
            };
            for k in 0..moon_used.len() {
                if k > 0 || sun_used.is_empty() && k > 0 {
                    e = e | moon_used[k].clone();
                }
            }
            for k in 0..sun_used.len() {
                e = e | sun_used[k].clone();
            }
            e
        };
        solver.add_expr(any_used);
    }

    // Single loop: all used cells are connected through used line edges.
    // (active_vertices_connected_2d would treat adjacent cells of separate
    // loops as connected, which wrongly accepts disjoint loops)
    let mut g = graph::infer_graph_from_2d_array((h, w));
    // infer_graph lists, for each cell in row-major order, the horizontal
    // adjacency edge first and the vertical one second.
    let mut edge_used = vec![];
    for y in 0..h {
        for x in 0..w {
            if x < w - 1 {
                edge_used.push(h_edge.at((y, x)).expr());
            }
            if y < h - 1 {
                edge_used.push(v_edge.at((y, x)).expr());
            }
        }
    }
    let b = &solver.bool_var_2d((h, w));
    for y in 0..h {
        for x in 0..w {
            solver.add_expr(b.at((y, x)).iff(cell_used(y, x)));
        }
    }
    graph::active_vertices_connected_via_active_edges(&mut solver, b, edge_used, &g);

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
            vec![vec![None; w - 1]; h],
            vec![vec![None; w]; h - 1],
            true,
        ));
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

fn encode_borders(bh: &[Vec<bool>], bv: &[Vec<bool>]) -> String {
    // pzpr encodes border bits 5 at a time in base 32 (most significant bit
    // first), horizontal borders first then vertical borders.
    let mut s = String::new();
    // Horizontal borders and vertical borders are encoded separately, each
    // padded to whole 5-bit groups.
    for chunk in bh.iter().flatten().cloned().collect::<Vec<_>>().chunks(5) {
        let mut v = 0i32;
        for &b in chunk {
            v = v * 2 + if b { 1 } else { 0 };
        }
        s.push_str(&base32(v));
    }
    for chunk in bv.iter().flatten().cloned().collect::<Vec<_>>().chunks(5) {
        let mut v = 0i32;
        for &b in chunk {
            v = v * 2 + if b { 1 } else { 0 };
        }
        s.push_str(&base32(v));
    }
    s
}

fn base32(mut v: i32) -> String {
    let digits = "0123456789abcdefghijklmnopqrstuv";
    if v == 0 {
        return "0".to_string();
    }
    let mut out = String::new();
    while v > 0 {
        out.push(digits.as_bytes()[(v % 32) as usize] as char);
        v /= 32;
    }
    out.chars().rev().collect()
}

fn decode_borders(s: &[u8], h: usize, w: usize) -> Option<(Vec<Vec<bool>>, Vec<Vec<bool>>)> {
    decode_borders_inner(s, h, w)
}

fn decode_bits_group(s: &[u8], n: usize) -> Option<Vec<bool>> {
    let n_chars = (n + 4) / 5;
    let mut bits = vec![];
    for &c in &s[..n_chars] {
        let val = match c {
            b'0'..=b'9' => (c - b'0') as i32,
            b'a'..=b'v' => (c - b'a' + 10) as i32,
            _ => return None,
        };
        for k in (0..5).rev() {
            bits.push((val >> k) & 1 == 1);
        }
    }
    bits.truncate(n);
    Some(bits)
}

fn decode_borders_inner(s: &[u8], h: usize, w: usize) -> Option<(Vec<Vec<bool>>, Vec<Vec<bool>>)> {
    let nh = h * (w - 1);
    let nv = (h - 1) * w;
    let n_chars_h = (nh + 4) / 5;
    let n_chars_v = (nv + 4) / 5;
    if s.len() < n_chars_h + n_chars_v {
        return None;
    }
    let bits_h = decode_bits_group(s, nh)?;
    let bits_v = decode_bits_group(&s[n_chars_h..], nv)?;
    let mut bh = vec![vec![false; w - 1]; h];
    let mut bv = vec![vec![false; w]; h - 1];
    let mut pos = 0;
    for row in bh.iter_mut() {
        for b in row.iter_mut() {
            *b = bits_h[pos];
            pos += 1;
        }
    }
    pos = 0;
    for row in bv.iter_mut() {
        for b in row.iter_mut() {
            *b = bits_v[pos];
            pos += 1;
        }
    }
    Some((bh, bv))
}

fn encode_circles(circles: &[Vec<Option<i32>>]) -> String {
    let mut s = String::new();
    let mut acc = 0i32;
    let mut cnt = 0;
    let mut flush = |acc: &mut i32, cnt: &mut usize, s: &mut String| {
        if *cnt > 0 {
            // pzpr pads the last group with zeros and encodes in base 27.
            let mut v = *acc;
            for _ in *cnt..3 {
                v *= 3;
            }
            s.push_str(&base27(v));
            *acc = 0;
            *cnt = 0;
        }
    };
    for row in circles {
        for &c in row {
            let v = match c {
                None => 0,
                Some(1) => 1,
                Some(2) => 2,
                _ => 0,
            };
            acc = acc * 3 + v;
            cnt += 1;
            if cnt == 3 {
                flush(&mut acc, &mut cnt, &mut s);
            }
        }
    }
    flush(&mut acc, &mut cnt, &mut s);
    s
}

fn base27(mut v: i32) -> String {
    let digits = "0123456789abcdefghijklmnopq";
    if v == 0 {
        return "0".to_string();
    }
    let mut out = String::new();
    while v > 0 {
        out.push(digits.as_bytes()[(v % 27) as usize] as char);
        v /= 27;
    }
    out.chars().rev().collect()
}

fn decode_circles(s: &[u8], h: usize, w: usize) -> Option<Vec<Vec<Option<i32>>>> {
    let mut circles = vec![vec![None; w]; h];
    let mut idx = 0;
    for &c in s {
        let val = match c {
            b'0'..=b'9' => (c - b'0') as i32,
            b'a'..=b'q' => (c - b'a' + 10) as i32,
            _ => return None,
        };
        for k in (0..3).rev() {
            if idx >= h * w {
                break;
            }
            let tri = 3i32.pow(k as u32);
            let v = val / tri;
            let (y, x) = (idx / w, idx % w);
            circles[y][x] = match v % 3 {
                1 => Some(1),
                2 => Some(2),
                _ => None,
            };
            idx += 1;
        }
        if idx >= h * w {
            break;
        }
    }
    Some(circles)
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = infer_shape(problem);
    if h == 0 || w == 0 {
        return None;
    }
    let borders = encode_borders(&problem.1, &problem.2);
    let circles = encode_circles(&problem.0);
    Some(format!(
        "https://puzz.link/p?dotchi2/{w}/{h}/{borders}{circles}"
    ))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let payload = url.split('?').nth(1)?;
    let mut parts = payload.split('/');
    if parts.next()? != "dotchi2" {
        return None;
    }
    let w = parts.next()?.parse().ok()?;
    let h = parts.next()?.parse().ok()?;
    let data = parts.next().unwrap_or("").as_bytes();
    let n_border_chars = (h * (w - 1) + 4) / 5 + ((h - 1) * w + 4) / 5;
    if data.len() < n_border_chars {
        return None;
    }
    let (bh, bv) = decode_borders(&data[..n_border_chars], h, w)?;
    let circles = decode_circles(&data[n_border_chars..], h, w)?;
    Some((circles, bh, bv))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_problem() -> Problem {
        // 3x3 with two regions split by a vertical border between column 0
        // and column 1.  Left region: one white circle; right region: one
        // black circle.  The loop passes straight through both (white) or
        // turns (black) - a tiny configuration used for serializer tests.
        let circles = vec![
            vec![None, Some(1), None],
            vec![None, None, None],
            vec![None, Some(2), None],
        ];
        let bh = vec![vec![false, false]; 3];
        let bv = vec![
            vec![true, false, false],
            vec![true, false, false],
        ];
        (circles, bh, bv)
    }

    #[test]
    fn test_dotchi2_serializer() {
        let problem = simple_problem();
        let url = serialize_problem(&problem).unwrap();
        let decoded = deserialize_problem(&url).unwrap();
        assert_eq!(decoded, problem);
    }

    #[test]
    fn test_dotchi2_pzpr_url() {
        // URL generated by the pzpr editor for: white circle at (0,1),
        // black circle at (2,1), vertical thick border between columns
        // 0 and 1.
        let url = "https://puzz.link/p?dotchi2/3/3/00i0306";
        let decoded = deserialize_problem(url).unwrap();
        assert_eq!(decoded.0[0][1], Some(1));
        assert_eq!(decoded.0[2][1], Some(2));
        assert_eq!(decoded.1, vec![vec![false; 2]; 3]);
        assert_eq!(
            decoded.2,
            vec![vec![true, false, false], vec![true, false, false]]
        );
        // And the same problem re-serializes to the same URL.
        assert_eq!(serialize_problem(&decoded).unwrap(), url);
    }

    #[test]
    fn test_white_circle_straight_black_turn() {
        // 3x3 single region with a white circle in the center.  Enumerate
        // all solutions and verify the line goes straight through the white
        // circle in every one of them.
        let circles = vec![
            vec![None, None, None],
            vec![None, Some(1), None],
            vec![None, None, None],
        ];
        let problem = (
            circles.clone(),
            vec![vec![false; 2]; 3],
            vec![vec![false; 3]; 2],
        );
        let (h, w) = (3usize, 3usize);

        // Rebuild the solver manually to enumerate answers.
        let mut solver = Solver::new();
        let h_edge = &solver.bool_var_2d((h, w - 1));
        let v_edge = &solver.bool_var_2d((h - 1, w));
        solver.add_answer_key_bool(h_edge);
        solver.add_answer_key_bool(v_edge);
        // degree constraints
        for y in 0..h {
            for x in 0..w {
                let mut d = vec![];
                if y > 0 {
                    d.push(v_edge.at((y - 1, x)));
                }
                if y < h - 1 {
                    d.push(v_edge.at((y, x)));
                }
                if x > 0 {
                    d.push(h_edge.at((y, x - 1)));
                }
                if x < w - 1 {
                    d.push(h_edge.at((y, x)));
                }
                if circles[y][x] == Some(1) {
                    let used = count_true(d.clone()).eq(2);
                    solver.add_expr(count_true(d.clone()).eq(0) | used.clone());
                    let horz = if x > 0 && x < w - 1 {
                        h_edge.at((y, x - 1)).expr() & h_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    let vert = if y > 0 && y < h - 1 {
                        v_edge.at((y - 1, x)).expr() & v_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    solver.add_expr(used.imp(horz | vert));
                } else {
                    solver.add_expr(count_true(d.clone()).eq(2) | count_true(d).eq(0));
                }
            }
        }
        // connectivity
        let mut used_cells = vec![];
        for y in 0..h {
            for x in 0..w {
                let mut d = vec![];
                if y > 0 {
                    d.push(v_edge.at((y - 1, x)).expr());
                }
                if y < h - 1 {
                    d.push(v_edge.at((y, x)).expr());
                }
                if x > 0 {
                    d.push(h_edge.at((y, x - 1)).expr());
                }
                if x < w - 1 {
                    d.push(h_edge.at((y, x)).expr());
                }
                let mut e = d[0].clone();
                for k in 1..d.len() {
                    e = e | d[k].clone();
                }
                used_cells.push(e);
            }
        }
        let used_2d = cspuz_rs::solver::BoolExprArray2D::new((h, w), used_cells);
        graph::active_vertices_connected_2d(&mut solver, &used_2d);

        let answers: Vec<_> = solver.answer_iter().take(50).collect();
        assert!(!answers.is_empty());
        for a in &answers {
            let hl = a.get_unwrap(&h_edge.at((1, 0)));
            let hr = a.get_unwrap(&h_edge.at((1, 1)));
            let vu = a.get_unwrap(&v_edge.at((0, 1)));
            let vd = a.get_unwrap(&v_edge.at((1, 1)));
            let used = [hl, hr, vu, vd].iter().filter(|&&b| b).count() == 2;
            if used {
                // straight: both horizontal or both vertical
                assert!(
                    (hl && hr) || (vu && vd),
                    "white circle turned: h=({},{}) v=({},{})",
                    hl,
                    hr,
                    vu,
                    vd
                );
            }
        }
    }

    #[test]
    fn test_black_circle_turns() {
        // 3x3 single region with a black circle in the center.  Every
        // solution must turn on the black circle.
        let circles = vec![
            vec![None, None, None],
            vec![None, Some(2), None],
            vec![None, None, None],
        ];
        let (h, w) = (3usize, 3usize);
        let mut solver = Solver::new();
        let h_edge = &solver.bool_var_2d((h, w - 1));
        let v_edge = &solver.bool_var_2d((h - 1, w));
        solver.add_answer_key_bool(h_edge);
        solver.add_answer_key_bool(v_edge);
        for y in 0..h {
            for x in 0..w {
                let mut d = vec![];
                if y > 0 {
                    d.push(v_edge.at((y - 1, x)));
                }
                if y < h - 1 {
                    d.push(v_edge.at((y, x)));
                }
                if x > 0 {
                    d.push(h_edge.at((y, x - 1)));
                }
                if x < w - 1 {
                    d.push(h_edge.at((y, x)));
                }
                if circles[y][x] == Some(2) {
                    let used = count_true(d.clone()).eq(2);
                    solver.add_expr(count_true(d.clone()).eq(0) | used.clone());
                    let horz = if x > 0 && x < w - 1 {
                        h_edge.at((y, x - 1)).expr() & h_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    let vert = if y > 0 && y < h - 1 {
                        v_edge.at((y - 1, x)).expr() & v_edge.at((y, x)).expr()
                    } else {
                        cspuz_rs::solver::FALSE
                    };
                    solver.add_expr(used.imp(!(horz | vert)));
                } else {
                    solver.add_expr(count_true(d.clone()).eq(2) | count_true(d).eq(0));
                }
            }
        }
        let mut used_cells = vec![];
        for y in 0..h {
            for x in 0..w {
                let mut d = vec![];
                if y > 0 {
                    d.push(v_edge.at((y - 1, x)).expr());
                }
                if y < h - 1 {
                    d.push(v_edge.at((y, x)).expr());
                }
                if x > 0 {
                    d.push(h_edge.at((y, x - 1)).expr());
                }
                if x < w - 1 {
                    d.push(h_edge.at((y, x)).expr());
                }
                let mut e = d[0].clone();
                for k in 1..d.len() {
                    e = e | d[k].clone();
                }
                used_cells.push(e);
            }
        }
        let used_2d = cspuz_rs::solver::BoolExprArray2D::new((h, w), used_cells);
        graph::active_vertices_connected_2d(&mut solver, &used_2d);

        let answers: Vec<_> = solver.answer_iter().take(50).collect();
        assert!(!answers.is_empty());
        for a in &answers {
            let hl = a.get_unwrap(&h_edge.at((1, 0)));
            let hr = a.get_unwrap(&h_edge.at((1, 1)));
            let vu = a.get_unwrap(&v_edge.at((0, 1)));
            let vd = a.get_unwrap(&v_edge.at((1, 1)));
            let used = [hl, hr, vu, vd].iter().filter(|&&b| b).count() == 2;
            if used {
                assert!(
                    !(hl && hr) && !(vu && vd),
                    "black circle went straight: h=({},{}) v=({},{})",
                    hl,
                    hr,
                    vu,
                    vd
                );
            }
        }
    }

    #[test]
    fn test_dotchi2_solver_runs() {
        let problem = simple_problem();
        let ans = solve_dotchi2(&problem);
        assert!(ans.is_some());
    }
}


#[cfg(test)]
mod conn_dbg {
    use cspuz_rs::graph;
    use cspuz_rs::solver::{BoolExprArray2D, Solver};

    #[test]
    fn test_conn_rejects_two_loops() {
        // 4x4: two separated 4-cell loops (left column ring and right column
        // ring).  active_vertices_connected_2d must reject this.
        let (h, w) = (4usize, 4usize);
        let mut solver = Solver::new();
        let h_edge = &solver.bool_var_2d((h, w - 1));
        let v_edge = &solver.bool_var_2d((h - 1, w));
        solver.add_answer_key_bool(h_edge);
        solver.add_answer_key_bool(v_edge);
        // 环1: 左列 (0,0)-(1,0)-(1,1)-(0,1)
        solver.add_expr(v_edge.at((0, 0)));
        solver.add_expr(h_edge.at((1, 0)));
        solver.add_expr(v_edge.at((0, 1)));
        solver.add_expr(h_edge.at((0, 0)));
        // 环2: 右列 (0,2)-(1,2)-(1,3)-(0,3)
        solver.add_expr(v_edge.at((0, 2)));
        solver.add_expr(h_edge.at((1, 2)));
        solver.add_expr(v_edge.at((0, 3)));
        solver.add_expr(h_edge.at((0, 2)));
        // 度数约束: 每格 0 或 2 条边 (让两个环成为唯一确定的解)
        for y in 0..h {
            for x in 0..w {
                let mut d = vec![];
                if y > 0 { d.push(v_edge.at((y - 1, x))); }
                if y < h - 1 { d.push(v_edge.at((y, x))); }
                if x > 0 { d.push(h_edge.at((y, x - 1))); }
                if x < w - 1 { d.push(h_edge.at((y, x))); }
                solver.add_expr(cspuz_rs::solver::count_true(d.clone()).eq(2) | cspuz_rs::solver::count_true(d).eq(0));
            }
        }
        // 活跃顶点 = 度>0 的格; 活跃边 = 使用的线边
        let mut g = graph::infer_graph_from_2d_array((h, w));
        let mut edge_used = vec![];
        for y in 0..h {
            for x in 0..w {
                if x < w - 1 { edge_used.push(h_edge.at((y, x)).expr()); }
                if y < h - 1 { edge_used.push(v_edge.at((y, x)).expr()); }
            }
        }
        let mut vertices = vec![];
        for y in 0..h {
            for x in 0..w {
                let mut d = vec![];
                if y > 0 { d.push(v_edge.at((y - 1, x)).expr()); }
                if y < h - 1 { d.push(v_edge.at((y, x)).expr()); }
                if x > 0 { d.push(h_edge.at((y, x - 1)).expr()); }
                if x < w - 1 { d.push(h_edge.at((y, x)).expr()); }
                let mut e = d[0].clone();
                for k in 1..d.len() { e = e | d[k].clone(); }
                vertices.push(e);
            }
        }
        graph::active_vertices_connected_via_active_edges(&mut solver, vertices, edge_used, &g);
        let answers: Vec<_> = solver.answer_iter().take(5).collect();
        eprintln!("DEBUG two-loops n_answers={}", answers.len());
        assert_eq!(answers.len(), 0, "two separated loops must be rejected");
    }
}


