use cspuz_rs::graph;
use cspuz_rs::serializer::{
    get_kudamono_url_info_detailed, parse_kudamono_dimension, problem_to_url_with_context,
    url_to_problem, Choice, Combinator, Context, ContextBasedGrid, DecInt, Dict, KudamonoBorder,
    KudamonoGrid, Number16, Optionalize, PrefixAndSuffix, Rooms, Size, Spaces, Tuple2,
};
use cspuz_rs::solver::{count_true, BoolVar, BoolVarArray2D, Solver};

pub fn add_constraints(
    solver: &mut Solver,
    has_light: &BoolVarArray2D,
    borders: &graph::InnerGridEdges<Vec<Vec<bool>>>,
    clues: &[Vec<Option<i32>>], // clue on a cell (not region)
    has_block: &[Vec<bool>],
) -> Option<()> {
    let (h, w) = borders.base_shape();

    for y in 0..h {
        for x in 0..w {
            if has_block[y][x] {
                solver.add_expr(!has_light.at((y, x)));
            }
        }
    }

    // 領域 = 太線で囲まれた領域。黒マスは領域を分割せず、領域内の
    // 黒マスには灯りを置けないだけ。
    let rooms = graph::borders_to_rooms(borders);
    for i in 0..rooms.len() {
        let mut clue: Option<i32> = None;

        for &(y, x) in &rooms[i] {
            if let Some(c) = clues[y][x] {
                if let Some(cc) = clue {
                    if cc != c {
                        return None;
                    }
                } else {
                    clue = Some(c);
                }
            }
        }

        let mut cells = vec![];
        for &pt in &rooms[i] {
            cells.push(has_light.at(pt));
        }

        if let Some(n) = clue {
            solver.add_expr(count_true(cells).eq(n));
        }
    }

    let mut horizontal_group: Vec<Vec<Option<BoolVar>>> = vec![vec![None; w]; h];
    for y in 0..h {
        let mut start: Option<usize> = None;
        for x in 0..=w {
            if x < w && !has_block[y][x] {
                if start.is_none() {
                    start = Some(x);
                }
            } else {
                if let Some(s) = start {
                    let v = solver.bool_var();
                    solver.add_expr(
                        has_light
                            .slice_fixed_y((y, s..x))
                            .count_true()
                            .eq(v.ite(1, 0)),
                    );
                    for x2 in s..x {
                        horizontal_group[y][x2] = Some(v.clone());
                    }
                    start = None;
                }
            }
        }
    }

    let mut vertical_group: Vec<Vec<Option<BoolVar>>> = vec![vec![None; w]; h];
    for x in 0..w {
        let mut start: Option<usize> = None;
        for y in 0..=h {
            if y < h && !has_block[y][x] {
                if start.is_none() {
                    start = Some(y);
                }
            } else {
                if let Some(s) = start {
                    let v = solver.bool_var();
                    solver.add_expr(
                        has_light
                            .slice_fixed_x((s..y, x))
                            .count_true()
                            .eq(v.ite(1, 0)),
                    );
                    for y2 in s..y {
                        vertical_group[y2][x] = Some(v.clone());
                    }
                    start = None;
                }
            }
        }
    }

    for y in 0..h {
        for x in 0..w {
            if !has_block[y][x] {
                solver.add_expr(
                    horizontal_group[y][x].as_ref().unwrap()
                        | vertical_group[y][x].as_ref().unwrap(),
                );
            }
        }
    }

    Some(())
}

pub fn solve_akari_region(
    borders: &graph::InnerGridEdges<Vec<Vec<bool>>>,
    clues: &[Vec<Option<i32>>], // clue on a cell (not region)
    has_block: &[Vec<bool>],
) -> Option<Vec<Vec<Option<bool>>>> {
    let (h, w) = borders.base_shape();

    let mut solver = Solver::new();
    let has_light = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(has_light);

    add_constraints(&mut solver, has_light, borders, clues, has_block)?;

    solver.irrefutable_facts().map(|f| f.get(has_light))
}

pub type Problem = (
    graph::InnerGridEdges<Vec<Vec<bool>>>,
    Vec<Vec<Option<i32>>>,
    Vec<Vec<bool>>,
);

//------------------------------------------------------------------------------
// pzprv3 URL format:
//   akari-regional/cols/rows/<cells>/<borders>
// <cells> encodes each cell with the pzpr "number16" scheme:
//   "."   = black cell (qnum = -2)
//   hex   = region number on a white cell
//   run   = ordinary white cells
// <borders> uses the standard pzpr border encoding (base-32, 5 cells/char).
//------------------------------------------------------------------------------

/// セル1マスの状態: None = 白マス, Some(-2) = 黒マス, Some(n>=0) = 数字付き白マス
fn cell_combinator() -> impl Combinator<Option<i32>> {
    Choice::new(vec![
        Box::new(Optionalize::new(Number16)),
        Box::new(Dict::new(Some(-2), ".")),
        Box::new(Spaces::new(None, 'g')),
    ])
}

fn pzpr_combinator() -> impl Combinator<(
    Vec<Vec<Option<i32>>>,
    graph::InnerGridEdges<Vec<Vec<bool>>>,
)> {
    Size::new(Tuple2::new(ContextBasedGrid::new(cell_combinator()), Rooms))
}

fn cells_from_problem(
    clues: &[Vec<Option<i32>>],
    has_block: &[Vec<bool>],
) -> Vec<Vec<Option<i32>>> {
    clues
        .iter()
        .zip(has_block.iter())
        .map(|(row_c, row_b)| {
            row_c
                .iter()
                .zip(row_b.iter())
                .map(|(&c, &b)| if b { Some(-2) } else { c })
                .collect()
        })
        .collect()
}

fn problem_from_cells(
    cells: &[Vec<Option<i32>>],
    borders: graph::InnerGridEdges<Vec<Vec<bool>>>,
) -> Option<Problem> {
    let (h, w) = borders.base_shape();
    if cells.len() != h || cells.iter().any(|row| row.len() != w) {
        return None;
    }
    let mut clues = vec![vec![None; w]; h];
    let mut has_block = vec![vec![false; w]; h];
    for y in 0..h {
        for x in 0..w {
            match cells[y][x] {
                None => (),
                Some(-2) => has_block[y][x] = true,
                Some(n) => clues[y][x] = Some(n),
            }
        }
    }
    Some((borders, clues, has_block))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    // pzprv3 format
    if let Some(problem) = url_to_problem(pzpr_combinator(), &["akari-regional"], url)
        .and_then(|(cells, borders)| problem_from_cells(&cells, borders))
    {
        return Some(problem);
    }

    // kudamono (paper-puzzle-player) format
    let parsed = get_kudamono_url_info_detailed(url)?;
    let (width, height) = parse_kudamono_dimension(parsed.get("W")?)?;

    let ctx = Context::sized_with_kudamono_mode(height, width, true);

    let clues;
    if let Some(p) = parsed.get("L-N") {
        let clues_combinator = KudamonoGrid::new(
            Optionalize::new(PrefixAndSuffix::new("(", DecInt, ")")),
            None,
        );
        clues = clues_combinator.deserialize(&ctx, p.as_bytes())?.1.pop()?;
    } else {
        clues = vec![vec![None; width]; height];
    }

    let has_block;
    if let Some(p) = parsed.get("L") {
        let block_combinator =
            KudamonoGrid::new(Choice::new(vec![Box::new(Dict::new(true, "z"))]), false);
        has_block = block_combinator.deserialize(&ctx, p.as_bytes())?.1.pop()?;
    } else {
        has_block = vec![vec![false; width]; height];
    }

    let border;
    if let Some(p) = parsed.get("SIE") {
        border = KudamonoBorder.deserialize(&ctx, p.as_bytes())?.1.pop()?;
    } else {
        border = graph::InnerGridEdges {
            horizontal: vec![vec![false; width]; height - 1],
            vertical: vec![vec![false; width - 1]; height],
        };
    }

    Some((border, clues, has_block))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (borders, clues, has_block) = problem;
    let (h, w) = borders.base_shape();

    problem_to_url_with_context(
        pzpr_combinator(),
        "akari-regional",
        (cells_from_problem(clues, has_block), borders.clone()),
        &Context::sized(h, w),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        // https://pedros.works/paper-puzzle-player?W=6x5&L=z7z6z8&L-N=(2)3(2)1(1)15(0)4&LF=g2g4g2g4g2g4g7&X=x22x2x2x1x2&SIE=9UL3UU9RURR1U4U5R&G=akari-regional

        let borders = graph::InnerGridEdges {
            horizontal: crate::util::tests::to_bool_2d([
                [1, 0, 0, 0, 1, 1],
                [0, 0, 0, 1, 0, 0],
                [0, 0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0, 1],
            ]),
            vertical: crate::util::tests::to_bool_2d([
                [0, 0, 1, 0, 0],
                [1, 0, 0, 1, 0],
                [0, 0, 0, 1, 0],
                [0, 1, 0, 0, 0],
                [0, 1, 0, 0, 0],
            ]),
        };

        let clues = vec![
            vec![Some(2), None, None, Some(1), None, None],
            vec![Some(2), None, None, None, Some(0), None],
            vec![None, None, None, None, None, None],
            vec![None, None, None, None, None, None],
            vec![None, None, None, None, None, None],
        ];

        let has_block = crate::util::tests::to_bool_2d([
            [0, 0, 0, 0, 0, 0],
            [0, 0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0, 0],
            [0, 0, 0, 0, 1, 0],
            [0, 0, 0, 0, 0, 0],
        ]);

        (borders, clues, has_block)
    }

    #[test]
    fn test_akari_regions_problem() {
        // 8x8: 十字の太線で4つの領域に分かれ、各領域の数字は4。
        // 黒マスは領域を分割しない。ユーザーの解(各領域4灯)が制約を
        // 満たすことを確認する。
        let url = "https://puzz.link/p?akari-regional/8/8/4i4l.i.h.h.i.j.g4i4k.i.j.i.g.i.h20g410820g4000001vo00000";
        let problem = deserialize_problem(url).expect("deserialize");
        let (h, w) = (8usize, 8usize);

        let lights: Vec<(usize, usize)> = vec![
            (0, 7),
            (1, 2),
            (1, 5),
            (2, 1),
            (2, 4),
            (2, 6),
            (3, 0),
            (3, 2),
            (4, 3),
            (5, 1),
            (5, 5),
            (5, 7),
            (6, 2),
            (6, 6),
            (7, 3),
            (7, 7),
        ];

        let mut solver = Solver::new();
        let has_light = &solver.bool_var_2d((h, w));
        solver.add_answer_key_bool(has_light);
        for y in 0..h {
            for x in 0..w {
                let on = lights.contains(&(y, x));
                if on {
                    solver.add_expr(has_light.at((y, x)));
                } else {
                    solver.add_expr(!has_light.at((y, x)));
                }
            }
        }
        add_constraints(&mut solver, has_light, &problem.0, &problem.1, &problem.2)
            .expect("clues consistent");
        assert!(solver.solve().is_some());
    }

    #[test]
    fn test_akari_regions_serializer_kudamono() {
        let problem = problem_for_tests();
        let url = "https://pedros.works/paper-puzzle-player?W=6x5&L=z7z6z8&L-N=(2)3(2)1(1)15(0)4&SIE=9UL3UU9RURR1U4U5R&G=akari-regional";
        assert_eq!(deserialize_problem(url), Some(problem));
    }

    #[test]
    fn test_akari_regions_serializer_pzpr() {
        let problem = problem_for_tests();
        let url = serialize_problem(&problem).expect("serialize");
        assert!(url.contains("akari-regional/"));
        let restored = deserialize_problem(&url).expect("deserialize");
        assert_eq!(restored, problem);
    }
}
