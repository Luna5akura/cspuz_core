use crate::board::{Board, BoardKind};
use crate::uniqueness::is_unique;
use cspuz_rs::graph;
use cspuz_rs::solver::{count_true, Solver, TRUE};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct DirectedClue {
    kind: i32,
    side: Side,
    value: i32,
}

pub struct TravelLineProblem {
    rows: usize,
    cols: usize,
    start: usize,
    goal: usize,
    start_outer_side: Option<Side>,
    goal_outer_side: Option<Side>,
    start_dir: Option<Side>,
    goal_dir: Option<Side>,
    bars: Vec<Vec<bool>>,
    ice: Vec<Vec<bool>>,
    cwfloor: Vec<Vec<bool>>,
    noadj: Vec<Vec<bool>>,
    notouch: Vec<Vec<bool>>,
    sloop: Vec<Vec<bool>>,
    specials: Vec<Vec<i32>>,
    order: Vec<Vec<i32>>,
    divide: Vec<Vec<i32>>,
    slither: Vec<Vec<i32>>,
    country_h: Vec<Vec<bool>>,
    country_v: Vec<Vec<bool>>,
    directed: Vec<Vec<Option<DirectedClue>>>,
    required_h: Vec<Vec<bool>>,
    required_v: Vec<Vec<bool>>,
    forced_h: Vec<Vec<i32>>,
    forced_v: Vec<Vec<i32>>,
}

fn parse_side(value: &str) -> Option<Side> {
    match value {
        "up" => Some(Side::Up),
        "down" => Some(Side::Down),
        "left" => Some(Side::Left),
        "right" => Some(Side::Right),
        _ => None,
    }
}

fn parse_optional_side(src: &json::JsonValue) -> Result<Option<Side>, &'static str> {
    if src.is_null() {
        return Ok(None);
    }
    let value = src.as_str().ok_or("invalid side value")?;
    parse_side(value).ok_or("invalid side value").map(Some)
}

fn parse_bool_grid(
    src: &json::JsonValue,
    rows: usize,
    cols: usize,
) -> Result<Vec<Vec<bool>>, &'static str> {
    if !src.is_array() || src.len() != rows {
        return Err("invalid boolean grid shape");
    }
    let mut ret = vec![vec![false; cols]; rows];
    for y in 0..rows {
        if !src[y].is_array() || src[y].len() != cols {
            return Err("invalid boolean grid shape");
        }
        for x in 0..cols {
            ret[y][x] = src[y][x].as_bool().ok_or("invalid boolean grid value")?;
        }
    }
    Ok(ret)
}

fn parse_directed_grid(
    src: &json::JsonValue,
    rows: usize,
    cols: usize,
) -> Result<Vec<Vec<Option<DirectedClue>>>, &'static str> {
    if !src.is_array() || src.len() != rows {
        return Err("invalid directed clue grid shape");
    }
    let mut ret = vec![vec![None; cols]; rows];
    for y in 0..rows {
        if !src[y].is_array() || src[y].len() != cols {
            return Err("invalid directed clue grid shape");
        }
        for x in 0..cols {
            let cell = &src[y][x];
            if cell.is_null() {
                continue;
            }
            if !cell.is_object() {
                return Err("invalid directed clue entry");
            }
            let kind = cell["kind"].as_i32().ok_or("invalid directed clue kind")?;
            let side =
                parse_side(cell["side"].as_str().ok_or("invalid directed clue side")?)
                    .ok_or("invalid directed clue side")?;
            let value = cell["value"].as_i32().ok_or("invalid directed clue value")?;
            ret[y][x] = Some(DirectedClue { kind, side, value });
        }
    }
    Ok(ret)
}

fn parse_optional_state_grid(
    src: &json::JsonValue,
    rows: usize,
    cols: usize,
) -> Result<Vec<Vec<i32>>, &'static str> {
    if src.is_null() {
        return Ok(vec![vec![-1; cols]; rows]);
    }
    if !src.is_array() || src.len() != rows {
        return Err("invalid forced state grid shape");
    }
    let mut ret = vec![vec![-1; cols]; rows];
    for y in 0..rows {
        if !src[y].is_array() || src[y].len() != cols {
            return Err("invalid forced state grid shape");
        }
        for x in 0..cols {
            let value = src[y][x].as_i32().ok_or("invalid forced state grid value")?;
            if !(-1..=1).contains(&value) {
                return Err("invalid forced state grid value");
            }
            ret[y][x] = value;
        }
    }
    Ok(ret)
}

pub fn deserialize_problem(payload: &str) -> Result<TravelLineProblem, &'static str> {
    let root = json::parse(payload).map_err(|_| "travelline payload JSON parsing failed")?;
    let rows = root["rows"].as_usize().ok_or("travelline rows missing")?;
    let cols = root["cols"].as_usize().ok_or("travelline cols missing")?;
    if rows == 0 || cols == 0 {
        return Err("travelline board shape is invalid");
    }
    let start = root["start"].as_usize().ok_or("travelline start missing")?;
    let goal = root["goal"].as_usize().ok_or("travelline goal missing")?;
    if start >= rows * cols || goal >= rows * cols {
        return Err("travelline start/goal out of range");
    }
    let start_outer_side = if root.has_key("startOuterSide") {
        parse_optional_side(&root["startOuterSide"]).map_err(|_| "travelline start outer side invalid")?
    } else if root["startSide"].is_null() {
        None
    } else {
        parse_optional_side(&root["startSide"]).map_err(|_| "travelline start side invalid")?
    };
    let goal_outer_side = if root.has_key("goalOuterSide") {
        parse_optional_side(&root["goalOuterSide"]).map_err(|_| "travelline goal outer side invalid")?
    } else if root["goalSide"].is_null() {
        None
    } else {
        parse_optional_side(&root["goalSide"]).map_err(|_| "travelline goal side invalid")?
    };
    let start_dir = if root.has_key("startDir") {
        parse_optional_side(&root["startDir"]).map_err(|_| "travelline start dir invalid")?
    } else {
        None
    };
    let goal_dir = if root.has_key("goalDir") {
        parse_optional_side(&root["goalDir"]).map_err(|_| "travelline goal dir invalid")?
    } else {
        None
    };
    if start_outer_side.is_none() && start_dir.is_none() {
        return Err("travelline start endpoint missing");
    }
    if goal_outer_side.is_none() && goal_dir.is_none() {
        return Err("travelline goal endpoint missing");
    }

    Ok(TravelLineProblem {
        rows,
        cols,
        start,
        goal,
        start_outer_side,
        goal_outer_side,
        start_dir,
        goal_dir,
        bars: parse_bool_grid(&root["bars"], rows, cols)?,
        ice: parse_bool_grid(&root["ice"], rows, cols)?,
        cwfloor: parse_bool_grid(&root["cwfloor"], rows, cols)?,
        noadj: parse_bool_grid(&root["noadj"], rows, cols)?,
        notouch: parse_bool_grid(&root["notouch"], rows, cols)?,
        sloop: parse_bool_grid(&root["sloop"], rows, cols)?,
        specials: {
            let src = &root["specials"];
            if !src.is_array() || src.len() != rows {
                return Err("invalid specials grid shape");
            }
            let mut ret = vec![vec![-1; cols]; rows];
            for y in 0..rows {
                if !src[y].is_array() || src[y].len() != cols {
                    return Err("invalid specials grid shape");
                }
                for x in 0..cols {
                    ret[y][x] = src[y][x].as_i32().ok_or("invalid specials grid value")?;
                }
            }
            ret
        },
        order: {
            let src = &root["order"];
            if !src.is_array() || src.len() != rows {
                return Err("invalid order grid shape");
            }
            let mut ret = vec![vec![-1; cols]; rows];
            for y in 0..rows {
                if !src[y].is_array() || src[y].len() != cols {
                    return Err("invalid order grid shape");
                }
                for x in 0..cols {
                    ret[y][x] = src[y][x].as_i32().ok_or("invalid order grid value")?;
                }
            }
            ret
        },
        divide: {
            let src = &root["divide"];
            if !src.is_array() || src.len() != rows + 1 {
                return Err("invalid divide grid shape");
            }
            let mut ret = vec![vec![0; cols + 1]; rows + 1];
            for y in 0..=rows {
                if !src[y].is_array() || src[y].len() != cols + 1 {
                    return Err("invalid divide grid shape");
                }
                for x in 0..=cols {
                    ret[y][x] = src[y][x].as_i32().ok_or("invalid divide grid value")?;
                }
            }
            ret
        },
        slither: {
            let src = &root["slither"];
            if !src.is_array() || src.len() != rows + 1 {
                return Err("invalid slither grid shape");
            }
            let mut ret = vec![vec![-1; cols + 1]; rows + 1];
            for y in 0..=rows {
                if !src[y].is_array() || src[y].len() != cols + 1 {
                    return Err("invalid slither grid shape");
                }
                for x in 0..=cols {
                    ret[y][x] = src[y][x].as_i32().ok_or("invalid slither grid value")?;
                }
            }
            ret
        },
        country_h: parse_bool_grid(&root["countryH"], rows, cols.saturating_sub(1))?,
        country_v: parse_bool_grid(&root["countryV"], rows.saturating_sub(1), cols)?,
        directed: parse_directed_grid(&root["directed"], rows, cols)?,
        required_h: parse_bool_grid(&root["requiredH"], rows, cols.saturating_sub(1))?,
        required_v: parse_bool_grid(&root["requiredV"], rows.saturating_sub(1), cols)?,
        forced_h: parse_optional_state_grid(&root["forcedH"], rows, cols.saturating_sub(1))?,
        forced_v: parse_optional_state_grid(&root["forcedV"], rows.saturating_sub(1), cols)?,
    })
}

fn neighbor_cell(y: usize, x: usize, rows: usize, cols: usize, side: Side) -> Option<(usize, usize)> {
    match side {
        Side::Up => (y > 0).then_some((y - 1, x)),
        Side::Down => (y + 1 < rows).then_some((y + 1, x)),
        Side::Left => (x > 0).then_some((y, x - 1)),
        Side::Right => (x + 1 < cols).then_some((y, x + 1)),
    }
}

fn opposite_side(side: Side) -> Side {
    match side {
        Side::Up => Side::Down,
        Side::Down => Side::Up,
        Side::Left => Side::Right,
        Side::Right => Side::Left,
    }
}

fn endpoint_outer_side(problem: &TravelLineProblem, idx: usize) -> Option<Side> {
    if idx == problem.start {
        problem.start_outer_side
    } else if idx == problem.goal {
        problem.goal_outer_side
    } else {
        None
    }
}

fn endpoint_has_outer_connector(problem: &TravelLineProblem, idx: usize) -> bool {
    endpoint_outer_side(problem, idx).is_some()
}

fn endpoint_allowed_inner_side(problem: &TravelLineProblem, idx: usize) -> Option<Side> {
    if idx == problem.start {
        problem.start_dir
    } else if idx == problem.goal {
        problem.goal_dir.map(opposite_side)
    } else {
        None
    }
}

fn side_expr(
    is_line: &graph::BoolGridEdges,
    y: usize,
    x: usize,
    rows: usize,
    cols: usize,
    side: Side,
    start: usize,
    goal: usize,
    start_outer_side: Option<Side>,
    goal_outer_side: Option<Side>,
) -> cspuz_rs::solver::BoolExpr {
    let idx = y * cols + x;
    match side {
        Side::Up => {
            if y > 0 {
                is_line.vertical.at((y - 1, x)).expr()
            } else if (idx == start && start_outer_side == Some(Side::Up))
                || (idx == goal && goal_outer_side == Some(Side::Up))
            {
                TRUE
            } else {
                !TRUE
            }
        }
        Side::Down => {
            if y + 1 < rows {
                is_line.vertical.at((y, x)).expr()
            } else if (idx == start && start_outer_side == Some(Side::Down))
                || (idx == goal && goal_outer_side == Some(Side::Down))
            {
                TRUE
            } else {
                !TRUE
            }
        }
        Side::Left => {
            if x > 0 {
                is_line.horizontal.at((y, x - 1)).expr()
            } else if (idx == start && start_outer_side == Some(Side::Left))
                || (idx == goal && goal_outer_side == Some(Side::Left))
            {
                TRUE
            } else {
                !TRUE
            }
        }
        Side::Right => {
            if x + 1 < cols {
                is_line.horizontal.at((y, x)).expr()
            } else if (idx == start && start_outer_side == Some(Side::Right))
                || (idx == goal && goal_outer_side == Some(Side::Right))
            {
                TRUE
            } else {
                !TRUE
            }
        }
    }
}

fn directional_cells(
    y: usize,
    x: usize,
    rows: usize,
    cols: usize,
    side: Side,
) -> Vec<(usize, usize)> {
    let mut ret = vec![];
    match side {
        Side::Up => {
            for yy in (0..y).rev() {
                ret.push((yy, x));
            }
        }
        Side::Down => {
            for yy in (y + 1)..rows {
                ret.push((yy, x));
            }
        }
        Side::Left => {
            for xx in (0..x).rev() {
                ret.push((y, xx));
            }
        }
        Side::Right => {
            for xx in (x + 1)..cols {
                ret.push((y, xx));
            }
        }
    }
    ret
}

#[derive(Clone, Copy)]
enum InnerEdgeRef {
    Horizontal(usize, usize),
    Vertical(usize, usize),
}

fn is_sparse_local_only(problem: &TravelLineProblem) -> bool {
    if problem
        .forced_h
        .iter()
        .flatten()
        .copied()
        .any(|v| v != -1)
        || problem
            .forced_v
            .iter()
            .flatten()
            .copied()
            .any(|v| v != -1)
    {
        return false;
    }
    for y in 0..problem.rows {
        for x in 0..problem.cols {
            if problem.ice[y][x]
                || problem.cwfloor[y][x]
                || problem.noadj[y][x]
                || problem.notouch[y][x]
                || problem.specials[y][x] != -1
                || problem.order[y][x] >= 0
                || problem.directed[y][x].is_some()
            {
                return false;
            }
        }
    }
    if problem
        .divide
        .iter()
        .flatten()
        .copied()
        .any(|v| v > 0)
        || problem
            .slither
            .iter()
            .flatten()
            .copied()
            .any(|v| v >= 0)
        || problem
            .country_h
            .iter()
            .flatten()
            .copied()
            .any(|v| v)
        || problem
            .country_v
            .iter()
            .flatten()
            .copied()
            .any(|v| v)
    {
        return false;
    }
    true
}

fn deduce_sparse_local_irrefutable(
    problem: &TravelLineProblem,
) -> Result<graph::BoolGridEdgesIrrefutableFacts, &'static str> {
    let rows = problem.rows;
    let cols = problem.cols;
    let mut horizontal = vec![vec![None; cols.saturating_sub(1)]; rows];
    let mut vertical = vec![vec![None; cols]; rows.saturating_sub(1)];

    fn set_edge(
        horizontal: &mut [Vec<Option<bool>>],
        vertical: &mut [Vec<Option<bool>>],
        edge: InnerEdgeRef,
        value: bool,
    ) -> Result<bool, &'static str> {
        let slot = match edge {
            InnerEdgeRef::Horizontal(y, x) => &mut horizontal[y][x],
            InnerEdgeRef::Vertical(y, x) => &mut vertical[y][x],
        };
        match *slot {
            Some(existing) if existing != value => Err("travelline local fast-path contradiction"),
            Some(_) => Ok(false),
            None => {
                *slot = Some(value);
                Ok(true)
            }
        }
    }

    fn incident_edges(
        y: usize,
        x: usize,
        rows: usize,
        cols: usize,
    ) -> Vec<InnerEdgeRef> {
        let mut ret = vec![];
        if y > 0 {
            ret.push(InnerEdgeRef::Vertical(y - 1, x));
        }
        if y + 1 < rows {
            ret.push(InnerEdgeRef::Vertical(y, x));
        }
        if x > 0 {
            ret.push(InnerEdgeRef::Horizontal(y, x - 1));
        }
        if x + 1 < cols {
            ret.push(InnerEdgeRef::Horizontal(y, x));
        }
        ret
    }

    for y in 0..rows {
        for x in 0..cols.saturating_sub(1) {
            if problem.required_h[y][x] {
                set_edge(&mut horizontal, &mut vertical, InnerEdgeRef::Horizontal(y, x), true)?;
            }
        }
    }
    for y in 0..rows.saturating_sub(1) {
        for x in 0..cols {
            if problem.required_v[y][x] {
                set_edge(&mut horizontal, &mut vertical, InnerEdgeRef::Vertical(y, x), true)?;
            }
        }
    }

    for y in 0..rows {
        for x in 0..cols {
            let idx = y * cols + x;
            let Some(allowed_side) = endpoint_allowed_inner_side(problem, idx) else {
                continue;
            };
            if !problem.bars[y][x] {
                continue;
            }
            let mut has_allowed_edge = false;
            for edge in incident_edges(y, x, rows, cols) {
                let is_allowed = matches!(
                    (edge, allowed_side),
                    (InnerEdgeRef::Vertical(yy, xx), Side::Up) if yy + 1 == y && xx == x
                ) || matches!(
                    (edge, allowed_side),
                    (InnerEdgeRef::Vertical(yy, xx), Side::Down) if yy == y && xx == x
                ) || matches!(
                    (edge, allowed_side),
                    (InnerEdgeRef::Horizontal(yy, xx), Side::Left) if yy == y && xx + 1 == x
                ) || matches!(
                    (edge, allowed_side),
                    (InnerEdgeRef::Horizontal(yy, xx), Side::Right) if yy == y && xx == x
                );
                if is_allowed {
                    has_allowed_edge = true;
                } else {
                    set_edge(&mut horizontal, &mut vertical, edge, false)?;
                }
            }
            if !has_allowed_edge {
                return Err("travelline endpoint bar direction invalid");
            }
        }
    }

    loop {
        let mut changed = false;
        for y in 0..rows {
            for x in 0..cols {
                let idx = y * cols + x;
                let mut exact_inner_degree = None;
                if problem.bars[y][x] && idx != problem.start && idx != problem.goal {
                    exact_inner_degree = Some(0);
                } else if problem.sloop[y][x] || idx == problem.start || idx == problem.goal {
                    exact_inner_degree = Some(if idx == problem.start || idx == problem.goal {
                        1
                    } else {
                        2
                    });
                }
                let Some(required_inner) = exact_inner_degree else {
                    continue;
                };
                let edges = incident_edges(y, x, rows, cols);
                let mut true_count = 0usize;
                let mut unknown = vec![];
                for edge in edges {
                    let state = match edge {
                        InnerEdgeRef::Horizontal(yy, xx) => horizontal[yy][xx],
                        InnerEdgeRef::Vertical(yy, xx) => vertical[yy][xx],
                    };
                    match state {
                        Some(true) => true_count += 1,
                        Some(false) => {}
                        None => unknown.push(edge),
                    }
                }

                if true_count > required_inner || true_count + unknown.len() < required_inner {
                    return Err("travelline local fast-path contradiction");
                }
                if true_count == required_inner {
                    for edge in unknown {
                        changed |= set_edge(&mut horizontal, &mut vertical, edge, false)?;
                    }
                } else if true_count + unknown.len() == required_inner {
                    for edge in unknown {
                        changed |= set_edge(&mut horizontal, &mut vertical, edge, true)?;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    Ok(graph::BoolGridEdgesIrrefutableFacts {
        horizontal,
        vertical,
    })
}

pub fn solve(problem: &TravelLineProblem) -> Result<Board, &'static str> {
    let rows = problem.rows;
    let cols = problem.cols;

    for y in 0..rows {
        for x in 0..cols {
            if let Some(clue) = problem.directed[y][x] {
                if clue.kind != 14 && clue.kind != 15 {
                    return Err("unsupported travelline clue kind");
                }
            }
            match problem.specials[y][x] {
                -1 | 3 | 4 | 7 | 8 => {}
                _ => return Err("unsupported travelline clue kind"),
            }
            if problem.order[y][x] < -1 {
                return Err("invalid order clue");
            }
        }
    }
    for y in 0..=rows {
        for x in 0..=cols {
            let n = problem.divide[y][x];
            if !(0..=3).contains(&n) {
                return Err("unsupported divide clue kind");
            }
        }
    }
    for y in 0..=rows {
        for x in 0..=cols {
            let n = problem.slither[y][x];
            if !(-1..=4).contains(&n) {
                return Err("unsupported slither clue kind");
            }
        }
    }

    if is_sparse_local_only(problem) {
        let line_facts = deduce_sparse_local_irrefutable(problem)?;
        let mut board = Board::new(BoardKind::Grid, rows, cols, is_unique(&line_facts));
        board.add_lines_irrefutable_facts(&line_facts, "green", None);
        return Ok(board);
    }

    let has_order = problem
        .order
        .iter()
        .flatten()
        .copied()
        .any(|v| v >= 0);
    let has_divide = problem
        .divide
        .iter()
        .flatten()
        .copied()
        .any(|v| v > 0);

    let mut solver = Solver::new();
    let is_line = &graph::BoolGridEdges::new(&mut solver, (rows - 1, cols - 1));
    let is_passed = &solver.bool_var_2d((rows, cols));
    let is_cross = &solver.bool_var_2d((rows, cols));
    let line_dir = Some(graph::BoolGridEdges::new(&mut solver, (rows - 1, cols - 1)));
    let max_rank = (rows * cols * 2) as i32;
    let rank = Some(solver.int_var_2d((rows, cols), 0, max_rank));
    let rank_cross_h = Some(solver.int_var_2d((rows, cols), 0, max_rank));
    let rank_cross_v = Some(solver.int_var_2d((rows, cols), 0, max_rank));
    let divide_type = if has_divide {
        Some(solver.int_var_2d((rows + 1, cols + 1), 0, 3))
    } else {
        None
    };
    solver.add_answer_key_bool(&is_line.horizontal);
    solver.add_answer_key_bool(&is_line.vertical);

    for y in 0..rows {
        for x in 0..(cols - 1) {
            match problem.forced_h[y][x] {
                1 => solver.add_expr(is_line.horizontal.at((y, x))),
                0 => solver.add_expr(!is_line.horizontal.at((y, x))),
                _ => {}
            }
        }
    }
    for y in 0..(rows - 1) {
        for x in 0..cols {
            match problem.forced_v[y][x] {
                1 => solver.add_expr(is_line.vertical.at((y, x))),
                0 => solver.add_expr(!is_line.vertical.at((y, x))),
                _ => {}
            }
        }
    }

    {
        let (edges, graph) = is_line.representation();
        let line_graph = graph.line_graph();
        graph::active_vertices_connected(&mut solver, &edges, &line_graph);
    }

    for y in 0..rows {
        for x in 0..cols {
            let idx = y * cols + x;
            let up = side_expr(
                is_line,
                y,
                x,
                rows,
                cols,
                Side::Up,
                problem.start,
                problem.goal,
                problem.start_outer_side,
                problem.goal_outer_side,
            );
            let down = side_expr(
                is_line,
                y,
                x,
                rows,
                cols,
                Side::Down,
                problem.start,
                problem.goal,
                problem.start_outer_side,
                problem.goal_outer_side,
            );
            let left = side_expr(
                is_line,
                y,
                x,
                rows,
                cols,
                Side::Left,
                problem.start,
                problem.goal,
                problem.start_outer_side,
                problem.goal_outer_side,
            );
            let right = side_expr(
                is_line,
                y,
                x,
                rows,
                cols,
                Side::Right,
                problem.start,
                problem.goal,
                problem.start_outer_side,
                problem.goal_outer_side,
            );
            let degree = count_true(vec![up.clone(), down.clone(), left.clone(), right.clone()]);
            let passed = is_passed.at((y, x));
            let vertical = up.clone() & down.clone();
            let horizontal = left.clone() & right.clone();
            let straight = vertical.clone() | horizontal.clone();
            let curve = passed.expr() & !vertical.clone() & !horizontal.clone();
            let mut inbound = vec![];
            let mut outbound = vec![];
            let mut inbound_up = !TRUE;
            let mut inbound_down = !TRUE;
            let mut inbound_left = !TRUE;
            let mut inbound_right = !TRUE;
            let mut outbound_up = !TRUE;
            let mut outbound_down = !TRUE;
            let mut outbound_left = !TRUE;
            let mut outbound_right = !TRUE;

            if let Some(line_dir) = &line_dir {
                if y > 0 {
                    inbound_up =
                        is_line.vertical.at((y - 1, x)) & line_dir.vertical.at((y - 1, x));
                    outbound_up =
                        is_line.vertical.at((y - 1, x)) & !line_dir.vertical.at((y - 1, x));
                    inbound.push(inbound_up.clone());
                    outbound.push(outbound_up.clone());
                    if let (Some(rank), Some(rank_cross_v)) = (&rank, &rank_cross_v) {
                        let rank_here_v =
                            is_cross.at((y, x)).ite(rank_cross_v.at((y, x)), rank.at((y, x)));
                        let rank_prev_down = is_cross
                            .at((y - 1, x))
                            .ite(rank_cross_v.at((y - 1, x)), rank.at((y - 1, x)));
                        solver.add_expr(
                            (is_line.vertical.at((y - 1, x))
                                & !line_dir.vertical.at((y - 1, x)))
                                .imp(rank_prev_down.eq(rank_here_v.clone() + 1)),
                        );
                        solver.add_expr(
                            (is_line.vertical.at((y - 1, x))
                                & line_dir.vertical.at((y - 1, x)))
                                .imp(rank_here_v.eq(rank_prev_down + 1)),
                        );
                    }
                }
                if y + 1 < rows {
                    inbound_down =
                        is_line.vertical.at((y, x)) & !line_dir.vertical.at((y, x));
                    outbound_down =
                        is_line.vertical.at((y, x)) & line_dir.vertical.at((y, x));
                    inbound.push(inbound_down.clone());
                    outbound.push(outbound_down.clone());
                }
                if x > 0 {
                    inbound_left =
                        is_line.horizontal.at((y, x - 1)) & line_dir.horizontal.at((y, x - 1));
                    outbound_left =
                        is_line.horizontal.at((y, x - 1)) & !line_dir.horizontal.at((y, x - 1));
                    inbound.push(inbound_left.clone());
                    outbound.push(outbound_left.clone());
                    if let (Some(rank), Some(rank_cross_h)) = (&rank, &rank_cross_h) {
                        let rank_here_h =
                            is_cross.at((y, x)).ite(rank_cross_h.at((y, x)), rank.at((y, x)));
                        let rank_prev_right = is_cross
                            .at((y, x - 1))
                            .ite(rank_cross_h.at((y, x - 1)), rank.at((y, x - 1)));
                        solver.add_expr(
                            (is_line.horizontal.at((y, x - 1))
                                & !line_dir.horizontal.at((y, x - 1)))
                                .imp(rank_prev_right.eq(rank_here_h.clone() + 1)),
                        );
                        solver.add_expr(
                            (is_line.horizontal.at((y, x - 1))
                                & line_dir.horizontal.at((y, x - 1)))
                                .imp(rank_here_h.eq(rank_prev_right + 1)),
                        );
                    }
                }
                if x + 1 < cols {
                    inbound_right =
                        is_line.horizontal.at((y, x)) & !line_dir.horizontal.at((y, x));
                    outbound_right =
                        is_line.horizontal.at((y, x)) & line_dir.horizontal.at((y, x));
                    inbound.push(inbound_right.clone());
                    outbound.push(outbound_right.clone());
                }
            }

            if problem.bars[y][x] && idx != problem.start && idx != problem.goal {
                solver.add_expr(!passed);
                solver.add_expr(!is_cross.at((y, x)));
                solver.add_expr(degree.eq(0));
                solver.add_expr(count_true(inbound).eq(0));
                solver.add_expr(count_true(outbound).eq(0));
                continue;
            }

            let is_yajilin_clue = matches!(
                problem.directed[y][x],
                Some(DirectedClue { kind: 14, .. })
            );

            if is_yajilin_clue {
                solver.add_expr(!passed.expr());
                solver.add_expr(!is_cross.at((y, x)));
                solver.add_expr(degree.eq(0));
                solver.add_expr(count_true(inbound).eq(0));
                solver.add_expr(count_true(outbound).eq(0));
            } else {
                if idx == problem.start || idx == problem.goal {
                    let endpoint_degree = if endpoint_has_outer_connector(problem, idx) {
                        2
                    } else {
                        1
                    };
                    solver.add_expr(degree.eq(passed.ite(endpoint_degree, 0)));
                } else {
                    solver.add_expr(degree.eq(is_cross.at((y, x)).ite(4, passed.ite(2, 0))));
                }
                if idx == problem.start {
                    solver.add_expr(count_true(inbound).eq(0));
                    solver.add_expr(count_true(outbound).eq(passed.ite(1, 0)));
                    if let Some(rank) = &rank {
                        solver.add_expr(rank.at((y, x)).eq(0));
                    }
                } else if idx == problem.goal {
                    solver.add_expr(count_true(inbound).eq(passed.ite(1, 0)));
                    solver.add_expr(count_true(outbound).eq(0));
                } else {
                    solver.add_expr(
                        count_true(inbound).eq(is_cross.at((y, x)).ite(2, passed.ite(1, 0))),
                    );
                    solver.add_expr(
                        count_true(outbound).eq(is_cross.at((y, x)).ite(2, passed.ite(1, 0))),
                    );
                }

                if problem.bars[y][x] {
                    if let Some(allowed_side) = endpoint_allowed_inner_side(problem, idx) {
                        let mut has_allowed_edge = false;
                        if y > 0 {
                            if allowed_side == Side::Up {
                                has_allowed_edge = true;
                            } else {
                                solver.add_expr(!is_line.vertical.at((y - 1, x)));
                            }
                        }
                        if y + 1 < rows {
                            if allowed_side == Side::Down {
                                has_allowed_edge = true;
                            } else {
                                solver.add_expr(!is_line.vertical.at((y, x)));
                            }
                        }
                        if x > 0 {
                            if allowed_side == Side::Left {
                                has_allowed_edge = true;
                            } else {
                                solver.add_expr(!is_line.horizontal.at((y, x - 1)));
                            }
                        }
                        if x + 1 < cols {
                            if allowed_side == Side::Right {
                                has_allowed_edge = true;
                            } else {
                                solver.add_expr(!is_line.horizontal.at((y, x)));
                            }
                        }
                        if !has_allowed_edge {
                            return Err("travelline endpoint bar direction invalid");
                        }
                    }
                }
            }

            if y == 0 || y + 1 == rows || x == 0 || x + 1 == cols {
                solver.add_expr(!is_cross.at((y, x)));
            } else if problem.ice[y][x] || problem.cwfloor[y][x] {
                if let (Some(rank_cross_h), Some(rank_cross_v)) = (&rank_cross_h, &rank_cross_v) {
                    solver.add_expr(
                        is_cross
                            .at((y, x))
                            .imp(rank_cross_h.at((y, x)).ne(rank_cross_v.at((y, x)))),
                    );
                }
                if let Some(line_dir) = &line_dir {
                    solver.add_expr(
                        is_cross
                            .at((y, x))
                            .imp(line_dir.vertical.at((y - 1, x)).iff(line_dir.vertical.at((y, x)))),
                    );
                    solver.add_expr(
                        is_cross.at((y, x)).imp(
                            line_dir
                                .horizontal
                                .at((y, x - 1))
                                .iff(line_dir.horizontal.at((y, x))),
                        ),
                    );
                }
            } else {
                solver.add_expr(!is_cross.at((y, x)));
            }

            if let Some(clue) = problem.directed[y][x] {
                let mut ray = vec![];
                for (yy, xx) in directional_cells(y, x, rows, cols, clue.side) {
                    if !problem.bars[yy][xx] {
                        ray.push(!is_passed.at((yy, xx)).expr());
                    }
                }
                if clue.kind == 14 {
                    solver.add_expr(count_true(ray).eq(clue.value));
                } else {
                    let mut segments = vec![];
                    for (yy, xx) in directional_cells(y, x, rows, cols, clue.side) {
                        match clue.side {
                            Side::Up => {
                                if yy + 1 <= y {
                                    segments.push(is_line.vertical.at((yy, xx)).expr());
                                }
                            }
                            Side::Down => {
                                if yy > y {
                                    segments.push(is_line.vertical.at((yy - 1, xx)).expr());
                                }
                            }
                            Side::Left => {
                                if xx + 1 <= x {
                                    segments.push(is_line.horizontal.at((yy, xx)).expr());
                                }
                            }
                            Side::Right => {
                                if xx > x {
                                    segments.push(is_line.horizontal.at((yy, xx - 1)).expr());
                                }
                            }
                        }
                    }
                    solver.add_expr(count_true(segments).eq(clue.value));
                }
            }

            match problem.specials[y][x] {
                3 => {
                    solver.add_expr(&passed);
                    solver.add_expr(straight.clone());
                    let mut cands = vec![];
                    if y > 0 {
                        if let Some((ny, nx)) = neighbor_cell(y, x, rows, cols, Side::Up) {
                            let nup = side_expr(
                                is_line, ny, nx, rows, cols, Side::Up, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let ndown = side_expr(
                                is_line, ny, nx, rows, cols, Side::Down, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nleft = side_expr(
                                is_line, ny, nx, rows, cols, Side::Left, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nright = side_expr(
                                is_line, ny, nx, rows, cols, Side::Right, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            cands.push(vertical.clone() & (is_passed.at((ny, nx)).expr() & !(nup & ndown) & !(nleft & nright)));
                        }
                    }
                    if y + 1 < rows {
                        if let Some((ny2, nx2)) = neighbor_cell(y, x, rows, cols, Side::Down) {
                            let nup2 = side_expr(
                                is_line, ny2, nx2, rows, cols, Side::Up, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let ndown2 = side_expr(
                                is_line, ny2, nx2, rows, cols, Side::Down, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nleft2 = side_expr(
                                is_line, ny2, nx2, rows, cols, Side::Left, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nright2 = side_expr(
                                is_line, ny2, nx2, rows, cols, Side::Right, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            cands.push(vertical.clone() & (is_passed.at((ny2, nx2)).expr() & !(nup2 & ndown2) & !(nleft2 & nright2)));
                        }
                    }
                    if x > 0 {
                        if let Some((ny3, nx3)) = neighbor_cell(y, x, rows, cols, Side::Left) {
                            let nup3 = side_expr(
                                is_line, ny3, nx3, rows, cols, Side::Up, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let ndown3 = side_expr(
                                is_line, ny3, nx3, rows, cols, Side::Down, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nleft3 = side_expr(
                                is_line, ny3, nx3, rows, cols, Side::Left, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nright3 = side_expr(
                                is_line, ny3, nx3, rows, cols, Side::Right, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            cands.push(horizontal.clone() & (is_passed.at((ny3, nx3)).expr() & !(nup3 & ndown3) & !(nleft3 & nright3)));
                        }
                    }
                    if x + 1 < cols {
                        if let Some((ny4, nx4)) = neighbor_cell(y, x, rows, cols, Side::Right) {
                            let nup4 = side_expr(
                                is_line, ny4, nx4, rows, cols, Side::Up, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let ndown4 = side_expr(
                                is_line, ny4, nx4, rows, cols, Side::Down, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nleft4 = side_expr(
                                is_line, ny4, nx4, rows, cols, Side::Left, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            let nright4 = side_expr(
                                is_line, ny4, nx4, rows, cols, Side::Right, problem.start, problem.goal,
                                problem.start_outer_side, problem.goal_outer_side
                            );
                            cands.push(horizontal.clone() & (is_passed.at((ny4, nx4)).expr() & !(nup4 & ndown4) & !(nleft4 & nright4)));
                        }
                    }
                    solver.add_expr(cspuz_rs::solver::any(cands));
                }
                4 => {
                    solver.add_expr(&passed);
                    solver.add_expr(curve.clone());
                }
                7 => {
                    solver.add_expr(&passed);
                    solver.add_expr(straight.clone());
                }
                8 => {
                    solver.add_expr(&passed);
                    solver.add_expr(curve.clone());
                }
                _ => {}
            }

            if problem.noadj[y][x] {
                if y + 1 < rows && problem.noadj[y + 1][x] {
                    solver.add_expr(!( !is_passed.at((y, x)) & !is_passed.at((y + 1, x)) ));
                }
                if x + 1 < cols && problem.noadj[y][x + 1] {
                    solver.add_expr(!( !is_passed.at((y, x)) & !is_passed.at((y, x + 1)) ));
                }
            }
            if problem.notouch[y][x] {
                if y + 1 < rows && problem.notouch[y + 1][x] {
                    solver.add_expr(
                        (is_passed.at((y, x)) & is_passed.at((y + 1, x)))
                            .imp(is_line.vertical.at((y, x)))
                    );
                }
                if x + 1 < cols && problem.notouch[y][x + 1] {
                    solver.add_expr(
                        (is_passed.at((y, x)) & is_passed.at((y, x + 1)))
                            .imp(is_line.horizontal.at((y, x)))
                    );
                }
            }
            if problem.sloop[y][x] {
                solver.add_expr(&passed);
            }

            if idx == problem.start || idx == problem.goal {
                solver.add_expr(&passed);
            }
            if problem.ice[y][x] {
                solver.add_expr((passed.expr() & !is_cross.at((y, x))).imp(straight.clone()));
            }
            if problem.cwfloor[y][x] {
                let right_turn = (inbound_up & outbound_right)
                    | (inbound_right & outbound_down)
                    | (inbound_down & outbound_left)
                    | (inbound_left & outbound_up);
                solver.add_expr((curve.clone() & !is_cross.at((y, x))).imp(right_turn));
            }
        }
    }

    for y in 0..rows {
        for x in 0..(cols - 1) {
            if problem.required_h[y][x] {
                solver.add_expr(is_line.horizontal.at((y, x)));
            }
        }
    }
    for y in 0..(rows - 1) {
        for x in 0..cols {
            if problem.required_v[y][x] {
                solver.add_expr(is_line.vertical.at((y, x)));
            }
            if problem.country_v[y][x] {
                solver.add_expr(is_passed.at((y, x)) | is_passed.at((y + 1, x)));
            }
        }
    }
    for y in 0..rows {
        for x in 0..(cols - 1) {
            if problem.country_h[y][x] {
                solver.add_expr(is_passed.at((y, x)) | is_passed.at((y, x + 1)));
            }
        }
    }
    for y in 0..=rows {
        for x in 0..=cols {
            let clue = problem.slither[y][x];
            if clue < 0 {
                continue;
            }
            let mut incident = vec![];
            if y > 0 && x < cols {
                incident.push(is_line.vertical.at((y - 1, x)).expr());
            }
            if y < rows - 1 && x < cols {
                incident.push(is_line.vertical.at((y, x)).expr());
            }
            if x > 0 && y < rows {
                incident.push(is_line.horizontal.at((y, x - 1)).expr());
            }
            if x < cols - 1 && y < rows {
                incident.push(is_line.horizontal.at((y, x)).expr());
            }
            solver.add_expr(count_true(incident).eq(clue));
        }
    }
    if has_order {
        for y in 0..rows {
            for x in 0..cols {
                if problem.order[y][x] >= 0 {
                    solver.add_expr(is_passed.at((y, x)));
                    solver.add_expr(!is_cross.at((y, x)));
                }
            }
        }
        let rank = rank.as_ref().unwrap();
        let mut ordered = vec![];
        for y in 0..rows {
            for x in 0..cols {
                if problem.order[y][x] >= 0 {
                    ordered.push((problem.order[y][x], y, x));
                }
            }
        }
        ordered.sort_by_key(|entry| entry.0);
        for i in 1..ordered.len() {
            let (_, py, px) = ordered[i - 1];
            let (_, cy, cx) = ordered[i];
            solver.add_expr(rank.at((py, px)).lt(rank.at((cy, cx))));
        }
    }
    if let Some(divide_type) = &divide_type {
        for y in 0..=rows {
            for x in 0..=cols {
                if problem.divide[y][x] > 0 {
                    solver.add_expr(divide_type.at((y, x)).eq(problem.divide[y][x]));
                }
                if y < rows - 1 && x < cols {
                    solver.add_expr(
                        (!is_line.vertical.at((y, x))).imp(
                            divide_type.at((y, x)).eq(divide_type.at((y + 1, x))),
                        ),
                    );
                }
                if x < cols - 1 && y < rows {
                    solver.add_expr(
                        (!is_line.horizontal.at((y, x))).imp(
                            divide_type.at((y, x)).eq(divide_type.at((y, x + 1))),
                        ),
                    );
                }
            }
        }
    }

    let facts = solver
        .irrefutable_facts()
        .ok_or("travelline backend found no solution")?;
    let line_facts = facts.get(is_line);

    let mut board = Board::new(BoardKind::Grid, rows, cols, is_unique(&line_facts));
    board.add_lines_irrefutable_facts(&line_facts, "green", None);
    Ok(board)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_travelline_backend_accepts_simple_open_path() {
        let payload = r#"{
            "rows": 2,
            "cols": 2,
            "start": 0,
            "goal": 1,
            "startSide": "up",
            "goalSide": "up",
            "bars": [[false,false],[false,false]],
            "ice": [[false,false],[false,false]],
            "cwfloor": [[false,false],[false,false]],
            "noadj": [[false,false],[false,false]],
            "notouch": [[false,false],[false,false]],
            "sloop": [[false,false],[false,false]],
            "specials": [[-1,-1],[-1,-1]],
            "order": [[-1,-1],[-1,-1]],
            "divide": [[0,0,0],[0,0,0],[0,0,0]],
            "slither": [[-1,-1,-1],[-1,-1,-1],[-1,-1,-1]],
            "countryH": [[false],[false]],
            "countryV": [[false,false]],
            "directed": [[null,null],[null,null]],
            "requiredH": [[false],[false]],
            "requiredV": [[false,false]]
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(board.is_ok(), "simple travelline backend puzzle should solve");
    }

    #[test]
    fn test_travelline_backend_respects_forced_line_states() {
        let payload = r#"{
            "rows": 1,
            "cols": 2,
            "start": 0,
            "goal": 1,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,false]],
            "ice": [[false,false]],
            "cwfloor": [[false,false]],
            "noadj": [[false,false]],
            "notouch": [[false,false]],
            "sloop": [[false,false]],
            "specials": [[-1,-1]],
            "order": [[-1,-1]],
            "divide": [[0,0,0],[0,0,0]],
            "slither": [[-1,-1,-1],[-1,-1,-1]],
            "countryH": [[false]],
            "countryV": [],
            "directed": [[null,null]],
            "requiredH": [[true]],
            "requiredV": [],
            "forcedH": [[0]],
            "forcedV": []
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(
            board.is_err(),
            "contradictory forced line states should make travelline unsatisfiable"
        );
    }

    #[test]
    fn test_travelline_backend_accepts_crossing_capable_floors() {
        let payload = r#"{
            "rows": 3,
            "cols": 3,
            "start": 3,
            "goal": 5,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,false,false],[false,false,false],[false,false,false]],
            "ice": [[false,false,false],[false,true,false],[false,false,false]],
            "cwfloor": [[false,false,false],[false,false,false],[false,false,false]],
            "noadj": [[false,false,false],[false,false,false],[false,false,false]],
            "notouch": [[false,false,false],[false,false,false],[false,false,false]],
            "sloop": [[false,false,false],[false,false,false],[false,false,false]],
            "specials": [[-1,-1,-1],[-1,-1,-1],[-1,-1,-1]],
            "order": [[-1,-1,-1],[-1,-1,-1],[-1,-1,-1]],
            "divide": [[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]],
            "slither": [[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "countryH": [[false,false],[false,false],[false,false]],
            "countryV": [[false,false,false],[false,false,false]],
            "directed": [[null,null,null],[null,null,null],[null,null,null]],
            "requiredH": [[false,false],[false,false],[false,false]],
            "requiredV": [[false,false,false],[false,false,false]]
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(board.is_ok(), "crossing-capable floor puzzle should solve");
    }

    #[test]
    fn test_travelline_backend_accepts_simple_order_without_crossing_floors() {
        let payload = r#"{
            "rows": 1,
            "cols": 3,
            "start": 0,
            "goal": 2,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,false,false]],
            "ice": [[false,false,false]],
            "cwfloor": [[false,false,false]],
            "noadj": [[false,false,false]],
            "notouch": [[false,false,false]],
            "sloop": [[false,false,false]],
            "specials": [[-1,-1,-1]],
            "order": [[0,-1,1]],
            "divide": [[0,0,0,0],[0,0,0,0]],
            "slither": [[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "countryH": [[false,false]],
            "countryV": [],
            "directed": [[null,null,null]],
            "requiredH": [[false,false]],
            "requiredV": []
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(board.is_ok(), "simple order puzzle should solve in backend");
    }

    #[test]
    fn test_travelline_backend_accepts_simple_required_line() {
        let payload = r#"{
            "rows": 1,
            "cols": 2,
            "start": 0,
            "goal": 1,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,false]],
            "ice": [[false,false]],
            "cwfloor": [[false,false]],
            "noadj": [[false,false]],
            "notouch": [[false,false]],
            "sloop": [[false,false]],
            "specials": [[-1,-1]],
            "order": [[-1,-1]],
            "divide": [[0,0,0],[0,0,0]],
            "slither": [[-1,-1,-1],[-1,-1,-1]],
            "countryH": [[false]],
            "countryV": [],
            "directed": [[null,null]],
            "requiredH": [[true]],
            "requiredV": []
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(board.is_ok(), "simple required-line puzzle should solve in backend");
    }

    #[test]
    fn test_travelline_backend_accepts_simple_yajilin_clue() {
        let payload = r#"{
            "rows": 2,
            "cols": 3,
            "start": 3,
            "goal": 5,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,false,false],[false,false,false]],
            "ice": [[false,false,false],[false,false,false]],
            "cwfloor": [[false,false,false],[false,false,false]],
            "noadj": [[false,false,false],[false,false,false]],
            "notouch": [[false,false,false],[false,false,false]],
            "sloop": [[false,false,false],[false,false,false]],
            "specials": [[-1,-1,-1],[-1,-1,-1]],
            "order": [[-1,-1,-1],[-1,-1,-1]],
            "divide": [[0,0,0,0],[0,0,0,0],[0,0,0,0]],
            "slither": [[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "countryH": [[false,false],[false,false]],
            "countryV": [[false,false,false]],
            "directed": [[{"kind":14,"side":"right","value":2},null,null],[null,null,null]],
            "requiredH": [[false,false],[false,false]],
            "requiredV": [[false,false,false]]
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(board.is_ok(), "simple yajilin-style clue should solve in backend");
    }

    #[test]
    fn test_travelline_backend_counts_other_yajilin_cells_and_skips_bars_in_ray_count() {
        let payload = r#"{
            "rows": 2,
            "cols": 4,
            "start": 4,
            "goal": 7,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,true,false,false],[false,false,false,false]],
            "ice": [[false,false,false,false],[false,false,false,false]],
            "cwfloor": [[false,false,false,false],[false,false,false,false]],
            "noadj": [[false,false,false,false],[false,false,false,false]],
            "notouch": [[false,false,false,false],[false,false,false,false]],
            "sloop": [[false,false,false,false],[false,false,false,false]],
            "specials": [[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "order": [[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "divide": [[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0]],
            "slither": [[-1,-1,-1,-1,-1],[-1,-1,-1,-1,-1],[-1,-1,-1,-1,-1]],
            "countryH": [[false,false,false],[false,false,false]],
            "countryV": [[false,false,false,false]],
            "directed": [[{"kind":14,"side":"right","value":2},null,{"kind":14,"side":"right","value":1},null],[null,null,null,null]],
            "requiredH": [[false,false,false],[false,false,false]],
            "requiredV": [[false,false,false,false]],
            "forcedH": [[-1,-1,-1],[-1,1,1]],
            "forcedV": [[-1,-1,-1,-1]]
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(
            board.is_ok(),
            "bar cells should be skipped while other yajilin clue cells are still counted"
        );
    }

    #[test]
    fn test_travelline_backend_accepts_simple_cw_clue() {
        let payload = r#"{
            "rows": 1,
            "cols": 3,
            "start": 0,
            "goal": 2,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,false,false]],
            "ice": [[false,false,false]],
            "cwfloor": [[false,false,false]],
            "noadj": [[false,false,false]],
            "notouch": [[false,false,false]],
            "sloop": [[false,false,false]],
            "specials": [[-1,-1,-1]],
            "order": [[-1,-1,-1]],
            "divide": [[0,0,0,0],[0,0,0,0]],
            "slither": [[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "countryH": [[false,false]],
            "countryV": [],
            "directed": [[{"kind":15,"side":"right","value":2},null,null]],
            "requiredH": [[false,false]],
            "requiredV": []
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(board.is_ok(), "simple clockwise-count clue should solve in backend");
    }

    #[test]
    fn test_travelline_backend_allows_start_or_goal_on_bar_cells() {
        let payload = r#"{
            "rows": 1,
            "cols": 2,
            "start": 0,
            "goal": 1,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[true,false]],
            "ice": [[false,false]],
            "cwfloor": [[false,false]],
            "noadj": [[false,false]],
            "notouch": [[false,false]],
            "sloop": [[false,false]],
            "specials": [[-1,-1]],
            "order": [[-1,-1]],
            "divide": [[0,0,0],[0,0,0]],
            "slither": [[-1,-1,-1],[-1,-1,-1]],
            "countryH": [[false]],
            "countryV": [],
            "directed": [[null,null]],
            "requiredH": [[true]],
            "requiredV": []
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(
            board.is_ok(),
            "a bar cell used as the start or goal endpoint should remain solvable"
        );
    }

    #[test]
    fn test_travelline_backend_supports_internal_bar_endpoints() {
        let payload = r#"{
            "rows": 1,
            "cols": 2,
            "start": 0,
            "goal": 1,
            "startSide": null,
            "goalSide": null,
            "startOuterSide": null,
            "goalOuterSide": null,
            "startDir": "right",
            "goalDir": "right",
            "bars": [[true,true]],
            "ice": [[false,false]],
            "cwfloor": [[false,false]],
            "noadj": [[false,false]],
            "notouch": [[false,false]],
            "sloop": [[false,false]],
            "specials": [[-1,-1]],
            "order": [[-1,-1]],
            "divide": [[0,0,0],[0,0,0]],
            "slither": [[-1,-1,-1],[-1,-1,-1]],
            "countryH": [[false]],
            "countryV": [],
            "directed": [[null,null]],
            "requiredH": [[true]],
            "requiredV": []
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(
            board.is_ok(),
            "internal bar endpoints with matching directions should be solvable in backend"
        );
    }

    #[test]
    fn test_travelline_backend_rejects_wrong_internal_edge_for_bar_endpoint() {
        let payload = r#"{
            "rows": 2,
            "cols": 2,
            "start": 0,
            "goal": 2,
            "startSide": null,
            "goalSide": "down",
            "startOuterSide": null,
            "goalOuterSide": "down",
            "startDir": "right",
            "goalDir": null,
            "bars": [[true,false],[false,false]],
            "ice": [[false,false],[false,false]],
            "cwfloor": [[false,false],[false,false]],
            "noadj": [[false,false],[false,false]],
            "notouch": [[false,false],[false,false]],
            "sloop": [[false,false],[false,false]],
            "specials": [[-1,-1],[-1,-1]],
            "order": [[-1,-1],[-1,-1]],
            "divide": [[0,0,0],[0,0,0],[0,0,0]],
            "slither": [[-1,-1,-1],[-1,-1,-1],[-1,-1,-1]],
            "countryH": [[false],[false]],
            "countryV": [[false,false]],
            "directed": [[null,null],[null,null]],
            "requiredH": [[false],[false]],
            "requiredV": [[true,false]],
            "forcedH": [[0],[ -1 ]],
            "forcedV": [[1,-1]]
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(
            board.is_err(),
            "a bar endpoint should not be allowed to use an internal edge that disagrees with its arrow direction"
        );
    }

    #[test]
    fn test_travelline_backend_accepts_order_with_crossing_floors_when_not_crossed() {
        let payload = r#"{
            "rows": 3,
            "cols": 3,
            "start": 3,
            "goal": 5,
            "startSide": "left",
            "goalSide": "right",
            "bars": [[false,false,false],[false,false,false],[false,false,false]],
            "ice": [[false,false,false],[false,true,false],[false,false,false]],
            "cwfloor": [[false,false,false],[false,false,false],[false,false,false]],
            "noadj": [[false,false,false],[false,false,false],[false,false,false]],
            "notouch": [[false,false,false],[false,false,false],[false,false,false]],
            "sloop": [[false,false,false],[false,false,false],[false,false,false]],
            "specials": [[-1,-1,-1],[-1,-1,-1],[-1,-1,-1]],
            "order": [[-1,-1,-1],[-1,0,1],[-1,-1,-1]],
            "divide": [[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]],
            "slither": [[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "countryH": [[false,false],[false,false],[false,false]],
            "countryV": [[false,false,false],[false,false,false]],
            "directed": [[null,null,null],[null,null,null],[null,null,null]],
            "requiredH": [[false,false],[false,false],[false,false]],
            "requiredV": [[false,false,false],[false,false,false]]
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let board = solve(&problem);
        assert!(board.is_ok(), "order clue should work with crossing-capable floors when not crossed");
    }

    #[test]
    fn test_travelline_sparse_local_fast_path_for_corner_sloop() {
        let payload = r#"{
            "rows": 4,
            "cols": 4,
            "start": 0,
            "goal": 15,
            "startSide": "up",
            "goalSide": "down",
            "bars": [[false,false,false,false],[false,false,false,false],[false,false,false,false],[false,false,false,false]],
            "ice": [[false,false,false,false],[false,false,false,false],[false,false,false,false],[false,false,false,false]],
            "cwfloor": [[false,false,false,false],[false,false,false,false],[false,false,false,false],[false,false,false,false]],
            "noadj": [[false,false,false,false],[false,false,false,false],[false,false,false,false],[false,false,false,false]],
            "notouch": [[false,false,false,false],[false,false,false,false],[false,false,false,false],[false,false,false,false]],
            "sloop": [[false,false,false,true],[false,false,false,false],[false,false,false,false],[false,false,false,false]],
            "specials": [[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "order": [[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1],[-1,-1,-1,-1]],
            "divide": [[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0]],
            "slither": [[-1,-1,-1,-1,-1],[-1,-1,-1,-1,-1],[-1,-1,-1,-1,-1],[-1,-1,-1,-1,-1],[-1,-1,-1,-1,-1]],
            "countryH": [[false,false,false],[false,false,false],[false,false,false],[false,false,false]],
            "countryV": [[false,false,false,false],[false,false,false,false],[false,false,false,false]],
            "directed": [[null,null,null,null],[null,null,null,null],[null,null,null,null],[null,null,null,null]],
            "requiredH": [[false,false,false],[false,false,false],[false,false,false],[false,false,false]],
            "requiredV": [[false,false,false,false],[false,false,false,false],[false,false,false,false]]
        }"#;

        let problem = deserialize_problem(payload).expect("payload should deserialize");
        let facts = deduce_sparse_local_irrefutable(&problem).expect("fast path should deduce");
        assert_eq!(facts.horizontal[0][2], Some(true));
        assert_eq!(facts.vertical[0][3], Some(true));
    }
}
