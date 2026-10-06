use cspuz_rs::graph;
use cspuz_rs::serializer::strip_prefix;
use cspuz_rs::solver::{BoolExpr, Solver, FALSE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HexMasyuClue {
    None,
    White,
    Black,
}

/// Directions of the edges incident to a hex cell (same convention as
/// `cspuz_rs::hex`):
/// - Right: (0, 1)
/// - Left: (0, -1)
/// - BottomLeft: (1, 0)
/// - BottomRight: (1, 1)
/// - TopRight: (-1, 0)
/// - TopLeft: (-1, -1)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HexDir {
    Right,
    Left,
    BottomLeft,
    BottomRight,
    TopRight,
    TopLeft,
}

impl HexDir {
    pub fn offset(self) -> (i32, i32) {
        match self {
            HexDir::Right => (0, 1),
            HexDir::Left => (0, -1),
            HexDir::BottomLeft => (1, 0),
            HexDir::BottomRight => (1, 1),
            HexDir::TopRight => (-1, 0),
            HexDir::TopLeft => (-1, -1),
        }
    }

    pub fn opposite(self) -> HexDir {
        match self {
            HexDir::Right => HexDir::Left,
            HexDir::Left => HexDir::Right,
            HexDir::BottomLeft => HexDir::TopRight,
            HexDir::BottomRight => HexDir::TopLeft,
            HexDir::TopRight => HexDir::BottomLeft,
            HexDir::TopLeft => HexDir::BottomRight,
        }
    }
}

pub const ALL_HEX_DIRS: [HexDir; 6] = [
    HexDir::Right,
    HexDir::Left,
    HexDir::BottomLeft,
    HexDir::BottomRight,
    HexDir::TopRight,
    HexDir::TopLeft,
];

/// The three straight axes through a hex cell (an axis with its two opposite
/// directions).
pub const HEX_AXES: [(HexDir, HexDir); 3] = [
    (HexDir::Right, HexDir::Left),
    (HexDir::BottomLeft, HexDir::TopRight),
    (HexDir::BottomRight, HexDir::TopLeft),
];

/// The board is a regular hexagon with side length `n` (in cells), represented
/// as a (2n - 1) x (2n - 1) rectangular grid whose cells outside the hexagon
/// are unused (cspuz's `hex` module convention).
///
/// The problem is a rectangular grid where `None` marks a cell outside the
/// hexagonal board and `Some(clue)` marks a cell inside it.
pub type Problem = Vec<Vec<Option<HexMasyuClue>>>;

/// The hexagonal board region of a (h, w) rectangular representation grid:
/// a cell (y, x) is on the board iff `-b < y - x < a` where the sides derive
/// from the grid dimensions (`a = w / 2 + 1`, `b = h / 2 + 1`; both rounded
/// down). For the regular hexagon with side `n` the grid is
/// `(2n - 1) x (2n - 1)` and `a = b = n`.
pub fn hex_sides(h: usize, w: usize) -> (i32, i32) {
    ((w / 2) as i32 + 1, (h / 2) as i32 + 1)
}

/// Whether (y, x) is a cell of the hexagonal board represented by a
/// (h, w) rectangular grid.
pub fn is_in_board(h: usize, w: usize, y: i32, x: i32) -> bool {
    if y < 0 || x < 0 || y as usize >= h || x as usize >= w {
        return false;
    }
    let (a, b) = hex_sides(h, w);
    let d = y - x;
    -b < d && d < a
}

/// The cells of the hexagonal board in row-major order of the representation
/// grid.
pub fn board_cells(h: usize, w: usize) -> Vec<(usize, usize)> {
    let mut ret = vec![];
    for y in 0..h {
        for x in 0..w {
            if is_in_board(h, w, y as i32, x as i32) {
                ret.push((y, x));
            }
        }
    }
    ret
}

/// Answer of a hexagonal masyu puzzle: for every edge shared by two adjacent
/// board cells, whether the loop uses it. The grids have the dimensions of the
/// rectangular representation; edges of cells outside the board are always
/// `Some(false)`.
pub struct HexMasyuAnswer {
    pub height: usize,
    pub width: usize,
    /// Edge (y, x)-(y, x + 1). Shape: (height, width - 1).
    pub right: Vec<Vec<Option<bool>>>,
    /// Edge (y, x)-(y + 1, x). Shape: (height - 1, width).
    pub bottom_left: Vec<Vec<Option<bool>>>,
    /// Edge (y, x)-(y + 1, x + 1). Shape: (height - 1, width - 1).
    pub bottom_right: Vec<Vec<Option<bool>>>,
}

pub fn solve_hex_masyu(clues: &Problem) -> Option<HexMasyuAnswer> {
    let h = clues.len();
    let w = clues[0].len();

    let mut solver = Solver::new();
    let is_right = &solver.bool_var_2d((h, w.saturating_sub(1)));
    let is_bl = &solver.bool_var_2d((h.saturating_sub(1), w));
    let is_br = &solver.bool_var_2d((h.saturating_sub(1), w.saturating_sub(1)));
    solver.add_answer_key_bool(is_right);
    solver.add_answer_key_bool(is_bl);
    solver.add_answer_key_bool(is_br);

    // The loop runs along the edges shared by adjacent board cells, so the
    // vertices of the loop graph are the board cells themselves.
    let mut idx_map = vec![vec![usize::MAX; w]; h];
    let mut num_vertices = 0;
    for y in 0..h {
        for x in 0..w {
            if is_in_board(h, w, y as i32, x as i32) {
                idx_map[y][x] = num_vertices;
                num_vertices += 1;
            }
        }
    }
    let mut g = graph::Graph::new(num_vertices);
    let mut edge_vars: Vec<cspuz_rs::solver::BoolVar> = vec![];
    for y in 0..h {
        for x in 0..w {
            if !is_in_board(h, w, y as i32, x as i32) {
                continue;
            }
            let idx = idx_map[y][x];
            if x + 1 < w && is_in_board(h, w, y as i32, (x + 1) as i32) {
                g.add_edge(idx, idx_map[y][x + 1]);
                edge_vars.push(is_right.at((y, x)).clone());
            }
            if y + 1 < h && is_in_board(h, w, (y + 1) as i32, x as i32) {
                g.add_edge(idx, idx_map[y + 1][x]);
                edge_vars.push(is_bl.at((y, x)).clone());
            }
            if y + 1 < h
                && x + 1 < w
                && is_in_board(h, w, (y + 1) as i32, (x + 1) as i32)
            {
                g.add_edge(idx, idx_map[y + 1][x + 1]);
                edge_vars.push(is_br.at((y, x)).clone());
            }
        }
    }
    graph::active_edges_single_cycle(&mut solver, &edge_vars, &g);

    // Edges that do not connect two board cells are forced off.
    for y in 0..h {
        for x in 0..w {
            if x + 1 < w
                && !(is_in_board(h, w, y as i32, x as i32)
                    && is_in_board(h, w, y as i32, (x + 1) as i32))
            {
                solver.add_expr(!is_right.at((y, x)));
            }
            if y + 1 < h
                && !(is_in_board(h, w, y as i32, x as i32)
                    && is_in_board(h, w, (y + 1) as i32, x as i32))
            {
                solver.add_expr(!is_bl.at((y, x)));
            }
            if y + 1 < h
                && x + 1 < w
                && !(is_in_board(h, w, y as i32, x as i32)
                    && is_in_board(h, w, (y + 1) as i32, (x + 1) as i32))
            {
                solver.add_expr(!is_br.at((y, x)));
            }
        }
    }

    // Edge accessor: returns the variable of the edge going out of (y, x) in
    // direction `dir`, or a false expression when there is no such edge
    // (outside the board).
    let edge = |y: i32, x: i32, dir: HexDir| -> BoolExpr {
        if !is_in_board(h, w, y, x) {
            return FALSE;
        }
        let (y, x) = (y as usize, x as usize);
        match dir {
            HexDir::Right => is_right.at_offset((y, x), (0, 0), false),
            HexDir::Left => is_right.at_offset((y, x), (0, -1), false),
            HexDir::BottomLeft => is_bl.at_offset((y, x), (0, 0), false),
            HexDir::BottomRight => is_br.at_offset((y, x), (0, 0), false),
            HexDir::TopRight => is_bl.at_offset((y, x), (-1, 0), false),
            HexDir::TopLeft => is_br.at_offset((y, x), (-1, -1), false),
        }
    };

    // The loop goes straight through (y, x) along the axis `(d1, d2)` iff both
    // edges of the axis are used.
    let straight_along = |y: i32, x: i32, (d1, d2): (HexDir, HexDir)| -> BoolExpr {
        edge(y, x, d1) & edge(y, x, d2)
    };

    for y in 0..h {
        for x in 0..w {
            let clue = match clues[y][x] {
                Some(clue) => clue,
                None => continue,
            };
            let (y, x) = (y as i32, x as i32);
            match clue {
                HexMasyuClue::None => (),
                HexMasyuClue::White => {
                    // The loop goes straight through the pearl, and it turns in
                    // at least one of the two cells entered next.
                    let mut e = FALSE;
                    for &(d1, d2) in &HEX_AXES {
                        let (dy1, dx1) = d1.offset();
                        let (dy2, dx2) = d2.offset();
                        // If both neighboring cells continue straight along the
                        // same axis, the loop turns in neither of them.
                        let both_neighbors_straight = straight_along(
                            y + dy1,
                            x + dx1,
                            (d1, d2),
                        ) & straight_along(y + dy2, x + dx2, (d1, d2));
                        e = e | (straight_along(y, x, (d1, d2)) & !both_neighbors_straight);
                    }
                    solver.add_expr(e);
                }
                HexMasyuClue::Black => {
                    // The loop turns in the pearl, and it goes straight through
                    // both cells entered next.
                    let mut e = FALSE;
                    for &d1 in &ALL_HEX_DIRS {
                        let (dy1, dx1) = d1.offset();
                        for &d2 in &ALL_HEX_DIRS {
                            if d2 == d1 || d2 == d1.opposite() {
                                continue;
                            }
                            let (dy2, dx2) = d2.offset();
                            let neighbor1_straight = HEX_AXES
                                .iter()
                                .map(|&(a1, a2)| straight_along(y + dy1, x + dx1, (a1, a2)))
                                .fold(FALSE, |acc, v| acc | v);
                            let neighbor2_straight = HEX_AXES
                                .iter()
                                .map(|&(a1, a2)| straight_along(y + dy2, x + dx2, (a1, a2)))
                                .fold(FALSE, |acc, v| acc | v);
                            e = e
                                | (edge(y, x, d1)
                                    & edge(y, x, d2)
                                    & neighbor1_straight
                                    & neighbor2_straight);
                        }
                    }
                    solver.add_expr(e);
                }
            }
        }
    }

    solver.irrefutable_facts().map(|f| {
        let mut right = vec![vec![None; w.saturating_sub(1)]; h];
        for y in 0..h {
            for x in 0..w.saturating_sub(1) {
                right[y][x] = f.get(&is_right.at((y, x)));
            }
        }
        let mut bottom_left = vec![vec![None; w]; h.saturating_sub(1)];
        for y in 0..h.saturating_sub(1) {
            for x in 0..w {
                bottom_left[y][x] = f.get(&is_bl.at((y, x)));
            }
        }
        let mut bottom_right = vec![vec![None; w.saturating_sub(1)]; h.saturating_sub(1)];
        for y in 0..h.saturating_sub(1) {
            for x in 0..w.saturating_sub(1) {
                bottom_right[y][x] = f.get(&is_br.at((y, x)));
            }
        }

        HexMasyuAnswer {
            height: h,
            width: w,
            right,
            bottom_left,
            bottom_right,
        }
    })
}

fn clue_to_int(clue: HexMasyuClue) -> i32 {
    match clue {
        HexMasyuClue::None => 0,
        HexMasyuClue::White => 1,
        HexMasyuClue::Black => 2,
    }
}

fn int_to_clue(n: i32) -> Option<HexMasyuClue> {
    match n {
        0 => Some(HexMasyuClue::None),
        1 => Some(HexMasyuClue::White),
        2 => Some(HexMasyuClue::Black),
        _ => None,
    }
}

fn encode_base27(values: &[i32]) -> String {
    let mut ret = String::new();
    let mut num = 0;
    let mut pass = 0;
    let tri = [9, 3, 1];
    for &v in values {
        pass += v * tri[num];
        num += 1;
        if num == 3 {
            ret.push(char::from_digit(pass as u32, 27).unwrap());
            num = 0;
            pass = 0;
        }
    }
    if num > 0 {
        ret.push(char::from_digit(pass as u32, 27).unwrap());
    }
    ret
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let h = problem.len();
    let w = problem[0].len();

    let mut values = vec![];
    for &(y, x) in &board_cells(h, w) {
        values.push(clue_to_int(problem[y][x]?));
    }
    let data = encode_base27(&values);

    Some(format!("https://puzz.link/p?hexmasyu/{}/{}/{}", w, h, data))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    let body = strip_prefix(url)?;
    let mut parts = body.split('/');
    let kind = parts.next()?;
    if !["hexmasyu", "hex-masyu", "hexmashu"].contains(&kind) {
        return None;
    }
    let w: usize = parts.next()?.parse().ok()?;
    let h: usize = parts.next()?.parse().ok()?;
    let data = parts.next()?;

    if w < 1 || h < 1 {
        return None;
    }
    let cells = board_cells(h, w);

    // None = 盤外, Some(HexMasyuClue::None) = 盤内の空きマス
    let mut problem = vec![vec![None; w]; h];
    for &(y, x) in &cells {
        problem[y][x] = Some(HexMasyuClue::None);
    }

    let tri = [9, 3, 1];
    let pos = ((cells.len() + 2) / 3).min(data.len());
    let mut c = 0;
    for i in 0..pos {
        let v = data.as_bytes()[i] as char;
        let v = v.to_digit(27)? as i32;
        for k in 0..3 {
            if c < cells.len() {
                let val = (v / tri[k]) % 3;
                let (y, x) = cells[c];
                problem[y][x] = Some(int_to_clue(val)?);
                c += 1;
            }
        }
    }
    Some(problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_edge(ans: &HexMasyuAnswer, y: i32, x: i32, dir: HexDir) -> Option<bool> {
        if !is_in_board(ans.height, ans.width, y, x) {
            return Some(false);
        }
        let (y, x) = (y as usize, x as usize);
        match dir {
            HexDir::Right => {
                if x + 1 < ans.width && is_in_board(ans.height, ans.width, y as i32, (x + 1) as i32)
                {
                    ans.right[y][x]
                } else {
                    Some(false)
                }
            }
            HexDir::Left => {
                if x >= 1 && is_in_board(ans.height, ans.width, y as i32, (x - 1) as i32) {
                    ans.right[y][x - 1]
                } else {
                    Some(false)
                }
            }
            HexDir::BottomLeft => {
                if y + 1 < ans.height
                    && is_in_board(ans.height, ans.width, (y + 1) as i32, x as i32)
                {
                    ans.bottom_left[y][x]
                } else {
                    Some(false)
                }
            }
            HexDir::BottomRight => {
                if y + 1 < ans.height
                    && x + 1 < ans.width
                    && is_in_board(ans.height, ans.width, (y + 1) as i32, (x + 1) as i32)
                {
                    ans.bottom_right[y][x]
                } else {
                    Some(false)
                }
            }
            HexDir::TopRight => {
                if y >= 1 && is_in_board(ans.height, ans.width, (y - 1) as i32, x as i32) {
                    ans.bottom_left[y - 1][x]
                } else {
                    Some(false)
                }
            }
            HexDir::TopLeft => {
                if y >= 1
                    && x >= 1
                    && is_in_board(ans.height, ans.width, (y - 1) as i32, (x - 1) as i32)
                {
                    ans.bottom_right[y - 1][x - 1]
                } else {
                    Some(false)
                }
            }
        }
    }

    fn empty_problem(n: usize) -> Problem {
        let h = 2 * n - 1;
        let w = 2 * n - 1;
        let mut ret = vec![vec![None; w]; h];
        for &(y, x) in &board_cells(h, w) {
            ret[y][x] = Some(HexMasyuClue::None);
        }
        ret
    }

    fn make_problem(h: usize, w: usize, pearls: &[(usize, usize, HexMasyuClue)]) -> Problem {
        let mut ret = vec![vec![None; w]; h];
        for &(y, x) in &board_cells(h, w) {
            ret[y][x] = Some(HexMasyuClue::None);
        }
        for &(y, x, clue) in pearls {
            assert!(is_in_board(h, w, y as i32, x as i32));
            ret[y][x] = Some(clue);
        }
        ret
    }

    // A complete loop as a set of (cell, dir) pairs (each used edge appears
    // exactly once, from the cell with the smaller repr coordinate order).
    fn loop_edges(cells: &[(i32, i32)]) -> Vec<((i32, i32), HexDir)> {
        let mut ret = vec![];
        let n = cells.len();
        for i in 0..n {
            let a = cells[i];
            let b = cells[(i + 1) % n];
            let dy = b.0 - a.0;
            let dx = b.1 - a.1;
            let dir = match (dy, dx) {
                (0, 1) => HexDir::Right,
                (0, -1) => HexDir::Left,
                (1, 0) => HexDir::BottomLeft,
                (1, 1) => HexDir::BottomRight,
                (-1, 0) => HexDir::TopRight,
                (-1, -1) => HexDir::TopLeft,
                _ => panic!("cells are not adjacent"),
            };
            ret.push((a, dir));
        }
        ret
    }

    // Validates that a complete loop satisfies the hexagonal masyu rules.
    fn check_loop_valid(h: usize, w: usize, problem: &Problem, loop_edges: &[((i32, i32), HexDir)]) {
        let mut edge_true = |y: i32, x: i32, dir: HexDir| -> bool {
            let (dy, dx) = dir.offset();
            let key = ((y, x), dir);
            if loop_edges.contains(&key) {
                return true;
            }
            // the same edge from the other side
            let ny = y + dy;
            let nx = x + dx;
            if is_in_board(h, w, ny, nx) {
                return loop_edges.contains(&((ny, nx), dir.opposite()));
            }
            false
        };

        // degree constraints and single cycle
        let mut visited = vec![vec![false; w]; h];
        let mut n_components = 0;
        for y0 in 0..h {
            for x0 in 0..w {
                if !is_in_board(h, w, y0 as i32, x0 as i32) {
                    continue;
                }
                let mut degree = 0;
                for &d in &ALL_HEX_DIRS {
                    if edge_true(y0 as i32, x0 as i32, d) {
                        degree += 1;
                    }
                }
                assert!(degree == 0 || degree == 2);
                if degree == 2 && !visited[y0][x0] {
                    n_components += 1;
                    let mut stack = vec![(y0, x0)];
                    visited[y0][x0] = true;
                    while let Some((y, x)) = stack.pop() {
                        for &d in &ALL_HEX_DIRS {
                            if !edge_true(y as i32, x as i32, d) {
                                continue;
                            }
                            let (dy, dx) = d.offset();
                            let ny = y as i32 + dy;
                            let nx = x as i32 + dx;
                            if !is_in_board(h, w, ny, nx) {
                                continue;
                            }
                            let (ny, nx) = (ny as usize, nx as usize);
                            if !visited[ny][nx] {
                                visited[ny][nx] = true;
                                stack.push((ny, nx));
                            }
                        }
                    }
                }
            }
        }
        assert!(n_components <= 1);

        // pearl rules
        for y in 0..h {
            for x in 0..w {
                let clue = match problem[y][x] {
                    Some(clue) => clue,
                    None => continue,
                };
                let used: Vec<HexDir> = ALL_HEX_DIRS
                    .iter()
                    .copied()
                    .filter(|&d| edge_true(y as i32, x as i32, d))
                    .collect();
                let straight = |y: i32, x: i32, (d1, d2): (HexDir, HexDir)| -> bool {
                    edge_true(y, x, d1) && edge_true(y, x, d2)
                };
                let turns = |y: i32, x: i32| -> bool {
                    !HEX_AXES.iter().any(|&(d1, d2)| straight(y, x, (d1, d2)))
                };
                match clue {
                    HexMasyuClue::None => (),
                    HexMasyuClue::White => {
                        assert_eq!(used.len(), 2);
                        assert!(HEX_AXES
                            .iter()
                            .any(|&(d1, d2)| straight(y as i32, x as i32, (d1, d2))));
                        assert!(used.iter().any(|&d| {
                            let (dy, dx) = d.offset();
                            turns(y as i32 + dy, x as i32 + dx)
                        }));
                    }
                    HexMasyuClue::Black => {
                        assert_eq!(used.len(), 2);
                        assert!(turns(y as i32, x as i32));
                        for &d in &used {
                            let (dy, dx) = d.offset();
                            let ny = y as i32 + dy;
                            let nx = x as i32 + dx;
                            assert!(HEX_AXES
                                .iter()
                                .any(|&(d1, d2)| straight(ny, nx, (d1, d2))));
                        }
                    }
                }
            }
        }
        let _ = &mut edge_true;
    }

    fn problem_for_tests() -> Problem {
        // The regular hexagonal board with side 3 (5x5 representation grid).
        // White pearls on the straight cells (0,1), (1,0) and a black pearl on
        // (0,0) which turns between two straight neighbors.
        let mut problem = empty_problem(3);
        problem[0][1] = Some(HexMasyuClue::White);
        problem[1][0] = Some(HexMasyuClue::White);
        problem[0][0] = Some(HexMasyuClue::Black);
        problem
    }

    #[test]
    fn test_hex_masyu_problem() {
        let problem = problem_for_tests();
        let ans = solve_hex_masyu(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        let (h, w) = (ans.height, ans.width);

        // A known valid loop for this problem:
        //   (0,2)-(0,1)-(0,0)-(1,0)-(2,0)-(2,1)-(1,1)-(1,2)-(0,2)
        let loop_cells: Vec<(i32, i32)> = vec![
            (0, 2),
            (0, 1),
            (0, 0),
            (1, 0),
            (2, 0),
            (2, 1),
            (1, 1),
            (1, 2),
        ];
        let loop_edges = loop_edges(&loop_cells);
        check_loop_valid(h, w, &problem, &loop_edges);

        // The solver's facts never contradict the known valid loop.
        for &((y, x), dir) in &loop_edges {
            assert!(get_edge(&ans, y, x, dir) != Some(false));
        }
        for y in 0..h {
            for x in 0..w {
                if !is_in_board(h, w, y as i32, x as i32) {
                    continue;
                }
                for &d in &ALL_HEX_DIRS {
                    let (dy, dx) = d.offset();
                    if !is_in_board(h, w, y as i32 + dy, x as i32 + dx) {
                        continue;
                    }
                    let on_loop = loop_edges.contains(&((y as i32, x as i32), d))
                        || loop_edges.contains(&((y as i32 + dy, x as i32 + dx), d.opposite()));
                    if !on_loop {
                        assert!(get_edge(&ans, y as i32, x as i32, d) != Some(true));
                    }
                }
            }
        }

        // The segment through the pearls is forced.
        assert_eq!(get_edge(&ans, 0, 2, HexDir::Left), Some(true)); // (0,2)-(0,1)
        assert_eq!(get_edge(&ans, 0, 1, HexDir::Left), Some(true)); // (0,1)-(0,0)
        assert_eq!(get_edge(&ans, 0, 0, HexDir::BottomLeft), Some(true)); // (0,0)-(1,0)
        assert_eq!(get_edge(&ans, 1, 0, HexDir::BottomLeft), Some(true)); // (1,0)-(2,0)
        // The two endpoints of the segment cannot use these edges.
        assert_eq!(get_edge(&ans, 0, 0, HexDir::BottomRight), Some(false)); // (0,0)-(1,1)
        assert_eq!(get_edge(&ans, 1, 0, HexDir::Right), Some(false)); // (1,0)-(1,1)
        // Edges that leave the hexagonal board are always off.
        assert_eq!(get_edge(&ans, 0, 2, HexDir::Right), Some(false)); // (0,2)-(0,3) 盤外
    }

    #[test]
    fn test_hex_masyu_invalid() {
        // A single white pearl at the top-left corner of the side-2 hexagon:
        // none of the three straight axes is available, so no loop can pass
        // straight through it.
        let mut white_only = empty_problem(2);
        white_only[0][0] = Some(HexMasyuClue::White);
        assert!(solve_hex_masyu(&white_only).is_none());
    }

    #[test]
    fn test_hex_masyu_serializer() {
        let problem = problem_for_tests();
        let encoded = serialize_problem(&problem).unwrap();
        assert!(encoded.contains("hexmasyu/5/5/"));
        let decoded = deserialize_problem(&encoded).unwrap();
        assert_eq!(decoded, problem);
    }

    #[test]
    fn test_hex_masyu_irregular_board() {
        // An irregular hexagonal board (6 rows x 5 columns): the region derives
        // from the grid dimensions, so a = 3, b = 4.
        let mut problem = make_problem(6, 5, &[]);
        problem[1][0] = Some(HexMasyuClue::White);
        problem[0][0] = Some(HexMasyuClue::Black);

        // The same pearls exist in the region.
        assert!(is_in_board(6, 5, 1, 0));
        assert!(is_in_board(6, 5, 0, 0));
        assert!(!is_in_board(6, 5, 5, 0));
        assert!(!is_in_board(6, 5, 0, 4));

        let ans = solve_hex_masyu(&problem);
        assert!(ans.is_some());

        let encoded = serialize_problem(&problem).unwrap();
        assert!(encoded.contains("hexmasyu/5/6/"));
        let decoded = deserialize_problem(&encoded).unwrap();
        assert_eq!(decoded, problem);
    }
}
