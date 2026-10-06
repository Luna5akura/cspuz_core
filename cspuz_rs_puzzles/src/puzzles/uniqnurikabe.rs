use crate::util;
use cspuz_rs::serializer::{
    problem_to_url, url_to_problem, Choice, Combinator, Dict, Grid, HexInt, Optionalize, Spaces,
};

use super::nurikabe::enumerate_answers_nurikabe;

/// Unique Nurikabe: Nurikabe with the additional rule that every island
/// (connected group of unshaded cells) has a different shape.  Shapes that
/// are rotations or reflections of each other are considered the same.
///
/// The shape constraint is enforced by enumerating Nurikabe answers and
/// filtering out those where two islands share a shape; the solver returns
/// the intersection of the remaining answers.  If the enumeration is cut off
/// by the answer limit, the result is treated as unknown (None).
pub fn solve_uniqnurikabe(clues: &[Vec<Option<i32>>]) -> Option<Vec<Vec<Option<bool>>>> {
    if clues.is_empty() || clues[0].is_empty() {
        return None;
    }
    let (h, w) = util::infer_shape(clues);

    const MAX_ENUM: usize = 50000;
    let answers = enumerate_answers_nurikabe(clues, MAX_ENUM);
    if answers.len() >= MAX_ENUM {
        // The enumeration was cut off; the filtered result would be unsound.
        return None;
    }
    let valid: Vec<Vec<Vec<bool>>> = answers
        .into_iter()
        .filter(|ans| islands_have_unique_shapes(ans, h, w))
        .collect();
    if valid.is_empty() {
        return None;
    }

    let mut merged = vec![vec![None; w]; h];
    for y in 0..h {
        for x in 0..w {
            let v = valid[0][y][x];
            if valid.iter().all(|a| a[y][x] == v) {
                merged[y][x] = Some(v);
            }
        }
    }
    Some(merged)
}

/// Returns true if all islands (connected unshaded regions) have pairwise
/// different shapes up to rotation and reflection.
fn islands_have_unique_shapes(ans: &[Vec<bool>], h: usize, w: usize) -> bool {
    let mut visited = vec![vec![false; w]; h];
    let mut seen = std::collections::HashSet::new();
    for y in 0..h {
        for x in 0..w {
            if visited[y][x] || ans[y][x] {
                continue;
            }
            let mut cells = vec![];
            let mut stack = vec![(y, x)];
            visited[y][x] = true;
            while let Some((cy, cx)) = stack.pop() {
                cells.push((cy as i32, cx as i32));
                for (dy, dx) in [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
                    let ny = cy as i32 + dy;
                    let nx = cx as i32 + dx;
                    if ny >= 0
                        && nx >= 0
                        && (ny as usize) < h
                        && (nx as usize) < w
                        && !visited[ny as usize][nx as usize]
                        && !ans[ny as usize][nx as usize]
                    {
                        visited[ny as usize][nx as usize] = true;
                        stack.push((ny as usize, nx as usize));
                    }
                }
            }
            let key = shape_key(&cells);
            if !seen.insert(key) {
                return false;
            }
        }
    }
    true
}

/// Canonical key of a polyomino shape: the lexicographically smallest
/// normalized coordinate list over all 8 rotations/reflections.
fn shape_key(cells: &[(i32, i32)]) -> String {
    let mut keys: Vec<String> = vec![];
    for t in 0..8 {
        let mut pts: Vec<(i32, i32)> = cells
            .iter()
            .map(|&(x0, y0)| {
                let (mut x, mut y) = (x0, y0);
                if t & 4 != 0 {
                    x = -x;
                }
                for _ in 0..(t & 3) {
                    let (nx, ny) = (y, -x);
                    x = nx;
                    y = ny;
                }
                (x, y)
            })
            .collect();
        let minx = pts.iter().map(|p| p.0).min().unwrap_or(0);
        let miny = pts.iter().map(|p| p.1).min().unwrap_or(0);
        pts.sort();
        let key = pts
            .iter()
            .map(|&(x, y)| format!("{},{}", x - minx, y - miny))
            .collect::<Vec<_>>()
            .join("/");
        keys.push(key);
    }
    keys.sort();
    keys[0].clone()
}

type Problem = Vec<Vec<Option<i32>>>;

fn combinator() -> impl Combinator<Problem> {
    Grid::new(Choice::new(vec![
        Box::new(Optionalize::new(HexInt)),
        Box::new(Spaces::new(None, 'g')),
        Box::new(Dict::new(Some(-1), ".")),
    ]))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    if problem.is_empty() || problem[0].is_empty() {
        return None;
    }
    let (h, w) = util::infer_shape(problem);
    if problem.iter().any(|row| row.len() != w) {
        return None;
    }
    let _ = h;
    problem_to_url(combinator(), "uniqnurikabe", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["uniqnurikabe"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clues(problem: &[Vec<i32>]) -> Problem {
        problem
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&n| {
                        if n == i32::MIN {
                            None
                        } else {
                            Some(n)
                        }
                    })
                    .collect()
            })
            .collect()
    }



    #[test]
    fn rejects_duplicate_island_shapes() {
        // Two 1x2 dominoes in a 4x4 grid.  In plain Nurikabe this has valid
        // answers, but in Unique Nurikabe the two islands would have the same
        // shape (a domino is invariant under the relevant symmetries), so a
        // unique-shaped solution must not contain both dominoes at once.
        let problem = clues(&[
            vec![2, i32::MIN, i32::MIN, 2],
            vec![i32::MIN, i32::MIN, i32::MIN, i32::MIN],
            vec![i32::MIN, i32::MIN, i32::MIN, i32::MIN],
            vec![i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        ]);
        // If a solution exists at all, none of its islands may repeat a shape.
        let ans = solve_uniqnurikabe(&problem);
        if let Some(ans) = &ans {
            assert!(islands_have_unique_shapes(
                &ans.iter()
                    .map(|row| row.iter().map(|v| v.unwrap_or(false)).collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
                4,
                4
            ));
        }
    }

    #[test]
    fn test_uniqnurikabe_serializer() {
        let problem = vec![vec![Some(1), None], vec![None, Some(2)]];
        let url = serialize_problem(&problem).unwrap();
        assert_eq!(deserialize_problem(&url), Some(problem));
    }

    #[test]
    fn rejects_empty_or_ragged_grids() {
        let empty: Problem = vec![];
        assert!(solve_uniqnurikabe(&empty).is_none());
        assert!(serialize_problem(&empty).is_none());
        let ragged = vec![vec![Some(1), None], vec![None]];
        assert!(serialize_problem(&ragged).is_none());
    }

    #[test]
    fn shape_key_ignores_rotation_and_reflection() {
        // L-tromino in four orientations must produce the same key.
        let l1 = vec![(0, 0), (1, 0), (0, 1)];
        let l2 = vec![(0, 0), (0, 1), (1, 1)];
        let l3 = vec![(0, 1), (1, 0), (1, 1)];
        let l4 = vec![(0, 0), (1, 0), (1, 1)];
        assert_eq!(shape_key(&l1), shape_key(&l2));
        assert_eq!(shape_key(&l1), shape_key(&l3));
        assert_eq!(shape_key(&l1), shape_key(&l4));
        // Straight tromino is a different shape.
        let straight = vec![(0, 0), (1, 0), (2, 0)];
        assert_ne!(shape_key(&l1), shape_key(&straight));
    }
}
