use crate::puzzles::statue_park::PiecesCombinator;
use crate::util;
use cspuz_rs::serializer::{
    url_to_problem, Choice, Combinator, ContextBasedGrid, Dict, HexInt, Optionalize, Size, Spaces,
    Tuple2,
};

type WireProblem = (Vec<Vec<Option<i32>>>, Vec<Vec<Vec<bool>>>);

fn combinator() -> impl Combinator<WireProblem> {
    Size::new(Tuple2::new(
        ContextBasedGrid::new(Choice::new(vec![
            Box::new(Optionalize::new(HexInt)),
            Box::new(Spaces::new(None, 'g')),
            Box::new(Dict::new(None, ".")),
        ])),
        PiecesCombinator,
    ))
}

pub fn deserialize_problem(url: &str) -> Option<(Vec<Vec<Option<i32>>>, Vec<Shape>)> {
    let (clues, pieces) = url_to_problem(
        combinator(),
        &["shapeminesweeper", "shape-minesweeper", "shape-minesweep"],
        url,
    )?;
    Some((
        clues,
        pieces.into_iter().map(|cells| Shape { cells }).collect(),
    ))
}
use cspuz_rs::solver::{any, count_true, Solver};

#[derive(Clone, Debug)]
pub struct Shape {
    pub cells: Vec<Vec<bool>>,
}

pub fn solve_shape_minesweeper(
    clues: &[Vec<Option<i32>>],
    shapes: &[Shape],
) -> Option<Vec<Vec<Option<bool>>>> {
    let (h, w) = util::infer_shape(clues);
    let mut s = Solver::new();
    let occ = &s.bool_var_2d((h, w));
    s.add_answer_key_bool(occ);
    let mut placements = Vec::new();
    let mut all_cells: Vec<Vec<Vec<(usize, usize)>>> = Vec::new();
    for sh in shapes {
        if sh.cells.is_empty()
            || sh.cells[0].is_empty()
            || sh.cells.iter().any(|row| row.len() != sh.cells[0].len())
        {
            return None;
        }
        let mut vars = Vec::new();
        let mut pcells = Vec::new();
        let orients = orientations(&sh.cells);
        for p in orients {
            let (ph, pw) = util::infer_shape(&p);
            for y in 0..=h.saturating_sub(ph) {
                for x in 0..=w.saturating_sub(pw) {
                    let v = s.bool_var();
                    let mut cells = Vec::new();
                    let mut ok = true;
                    for py in 0..ph {
                        for px in 0..pw {
                            if p[py][px] {
                                let yy = y + py;
                                let xx = x + px;
                                if clues[yy][xx].is_some() {
                                    ok = false;
                                }
                                cells.push((yy, xx));
                            }
                        }
                    }
                    if !ok {
                        s.add_expr(!v);
                        continue;
                    }
                    for &(yy, xx) in &cells {
                        s.add_expr(v.imp(occ.at((yy, xx))));
                    }
                    vars.push(v);
                    pcells.push(cells);
                }
            }
        }
        s.add_expr(count_true(vars.clone()).eq(1));
        placements.push(vars);
        all_cells.push(pcells);
    }
    for i in 0..placements.len() {
        for j in (i + 1)..placements.len() {
            for (a, va) in placements[i].iter().enumerate() {
                for (b, vb) in placements[j].iter().enumerate() {
                    let touch = all_cells[i][a].iter().any(|&(y, x)| {
                        all_cells[j][b].iter().any(|&(yy, xx)| {
                            (y as i32 - yy as i32).abs() <= 1 && (x as i32 - xx as i32).abs() <= 1
                        })
                    });
                    if touch {
                        s.add_expr(va.clone().imp(!vb.clone()));
                    }
                }
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut vs = Vec::new();
            for i in 0..all_cells.len() {
                for (k, cells) in all_cells[i].iter().enumerate() {
                    if cells.iter().any(|&(yy, xx)| yy == y && xx == x) {
                        vs.push(placements[i][k].clone());
                    }
                }
            }
            if !vs.is_empty() {
                s.add_expr(occ.at((y, x)).iff(any(vs)));
            } else {
                s.add_expr(!occ.at((y, x)));
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            if let Some(n) = clues[y][x] {
                let mut ns = Vec::new();
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dy == 0 && dx == 0 {
                            continue;
                        };
                        let yy = y as i32 + dy;
                        let xx = x as i32 + dx;
                        if yy >= 0 && xx >= 0 && yy < h as i32 && xx < w as i32 {
                            ns.push(occ.at((yy as usize, xx as usize)));
                        }
                    }
                }
                s.add_expr(count_true(ns).eq(n));
                s.add_expr(!occ.at((y, x)));
            }
        }
    }
    s.irrefutable_facts().map(|f| f.get(occ))
}

fn orientations(p: &[Vec<bool>]) -> Vec<Vec<Vec<bool>>> {
    let mut r = Vec::new();
    let mut q = p.to_vec();
    for _ in 0..4 {
        for f in [false, true] {
            let mut a = q.clone();
            if f {
                for row in &mut a {
                    row.reverse();
                }
            }
            if !r.contains(&a) {
                r.push(a);
            }
        }
        let h = q.len();
        let w = q[0].len();
        let mut n = vec![vec![false; h]; w];
        for y in 0..h {
            for x in 0..w {
                n[x][h - 1 - y] = q[y][x];
            }
        }
        q = n;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decode_standard_piece_bank() {
        let (clues, shapes) =
            deserialize_problem("https://puzz.link/p?shapeminesweeper/4/4/................//t")
                .unwrap();
        assert_eq!(clues.len(), 4);
        assert!(!shapes.is_empty());
    }

    #[test]
    fn zero_is_a_numeric_clue() {
        let (clues, _) =
            deserialize_problem("https://puzz.link/p?shapeminesweeper/4/4/0...............//t")
                .unwrap();
        assert_eq!(clues[0][0], Some(0));
    }
}
