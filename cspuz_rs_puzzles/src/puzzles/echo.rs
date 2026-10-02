use crate::util;
use cspuz_rs::graph;
use cspuz_rs::serializer::{
    from_base16, problem_to_url, to_base16, url_to_problem, Choice, Combinator, Context, Grid,
    Optionalize, Spaces,
};
use cspuz_rs::solver::{any, consecutive_prefix_true, int_constant, Solver};

pub fn solve_echo(clues: &[Vec<Option<Vec<i32>>>]) -> Option<Vec<Vec<Option<bool>>>> {
    let (h, w) = util::infer_shape(clues);

    let mut solver = Solver::new();
    let is_black = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(is_black);

    // Black cells must not form a 2x2 block.
    if h >= 2 && w >= 2 {
        solver.add_expr(!(is_black.conv2d_and((2, 2))));
    }

    // All black cells must be orthogonally connected.
    graph::active_vertices_connected_2d(&mut solver, is_black);

    // The distance to a black cell in a direction never exceeds the board size.
    let max_dist = h.max(w) as i32;

    for y in 0..h {
        for x in 0..w {
            let entries = match &clues[y][x] {
                Some(entries) if !entries.is_empty() => entries,
                _ => continue,
            };

            // A cell with a clue must not be shaded.
            solver.add_expr(!is_black.at((y, x)));

            // The clue shows a *set* of distances, so duplicate numbers count
            // only once, while each "?" (-1) stands for a distinct wildcard.
            // All elements of the set are pairwise distinct.
            let mut nums: Vec<i32> = entries.iter().copied().filter(|&e| e >= 0).collect();
            nums.sort_unstable();
            nums.dedup();
            let num_unknowns = entries.iter().filter(|&&e| e < 0).count();

            let mut entry_exprs: Vec<cspuz_rs::solver::IntExpr> = Vec::new();
            for &n in &nums {
                entry_exprs.push(int_constant(n));
            }
            for _ in 0..num_unknowns {
                let v = solver.int_var(1, max_dist);
                entry_exprs.push(v.expr());
            }

            for i in 0..entry_exprs.len() {
                for j in (i + 1)..entry_exprs.len() {
                    solver.add_expr(entry_exprs[i].ne(entry_exprs[j].clone()));
                }
            }

            // Four rays starting from the clue cell, ordered from the nearest
            // cell to the farthest.
            let rays: Vec<cspuz_rs::solver::BoolVarArray1D> = vec![
                is_black.slice_fixed_x((..y, x)).reverse(),
                is_black.slice_fixed_x(((y + 1).., x)),
                is_black.slice_fixed_y((y, ..x)).reverse(),
                is_black.slice_fixed_y((y, (x + 1)..)),
            ];

            // For each ray, "prefix" is the number of consecutive white cells
            // from the clue cell outward. If a black cell exists on the ray,
            // its distance is prefix + 1; otherwise the ray is inactive
            // (prefix == ray length).
            let ray_info: Vec<(cspuz_rs::solver::IntExpr, cspuz_rs::solver::BoolExpr)> = rays
                .iter()
                .map(|ray| {
                    let prefix = consecutive_prefix_true(!ray);
                    let active = prefix.lt(ray.len() as i32);
                    (prefix, active)
                })
                .collect();

            // Every element of the set must be realized by at least one ray.
            for e in &entry_exprs {
                let terms = ray_info
                    .iter()
                    .map(|(prefix, active)| (active.clone() & (prefix + 1).eq(e.clone())))
                    .collect::<Vec<_>>();
                solver.add_expr(any(terms));
            }

            // If a ray contains a black cell, its distance must be one of the
            // elements of the set.
            for (prefix, active) in &ray_info {
                let terms = entry_exprs
                    .iter()
                    .map(|e| (prefix + 1).eq(e.clone()))
                    .collect::<Vec<_>>();
                solver.add_expr(active.imp(any(terms)));
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(is_black))
}

pub type Problem = Vec<Vec<Option<Vec<i32>>>>;

/// Encodes one clue cell: each entry is either a number in 1..=4095
/// (single hex digit, or "-xx", or "+xxx") or a "?" (encoded as "."),
/// followed by "0" as a terminator.
struct EchoClueCombinator;

impl Combinator<Vec<i32>> for EchoClueCombinator {
    fn serialize(&self, _: &Context, input: &[Vec<i32>]) -> Option<(usize, Vec<u8>)> {
        if input.is_empty() || input[0].is_empty() {
            return None;
        }

        let mut entries = input[0].clone();
        entries.sort_unstable();

        let mut ret = vec![];
        for &e in &entries {
            if e < 0 {
                ret.push(b'.');
            } else if (1..16).contains(&e) {
                ret.push(to_base16(e));
            } else if (16..256).contains(&e) {
                ret.push(b'-');
                ret.push(to_base16(e >> 4));
                ret.push(to_base16(e & 15));
            } else if (256..4096).contains(&e) {
                ret.push(b'+');
                ret.push(to_base16(e >> 8));
                ret.push(to_base16((e >> 4) & 15));
                ret.push(to_base16(e & 15));
            } else {
                return None;
            }
        }
        ret.push(b'0');

        Some((1, ret))
    }

    fn deserialize(&self, _: &Context, input: &[u8]) -> Option<(usize, Vec<Vec<i32>>)> {
        if input.is_empty() {
            return None;
        }

        let mut i = 0;
        let mut entries = vec![];
        loop {
            if i >= input.len() {
                return None;
            }
            let c = input[i];
            if c == b'0' {
                i += 1;
                break;
            }
            let e = if c == b'.' {
                i += 1;
                -1
            } else if c == b'-' {
                if i + 2 >= input.len() {
                    return None;
                }
                let v = (from_base16(input[i + 1])? << 4) | from_base16(input[i + 2])?;
                i += 3;
                v
            } else if c == b'+' {
                if i + 3 >= input.len() {
                    return None;
                }
                let v = (from_base16(input[i + 1])? << 8)
                    | (from_base16(input[i + 2])? << 4)
                    | from_base16(input[i + 3])?;
                i += 4;
                v
            } else {
                let v = from_base16(c)?;
                i += 1;
                v
            };
            if e == 0 {
                // "0" is only valid as the terminator.
                return None;
            }
            entries.push(e);
        }
        entries.sort_unstable();
        Some((i, vec![entries]))
    }
}

fn combinator() -> impl Combinator<Problem> {
    Grid::new(Choice::new(vec![
        Box::new(Optionalize::new(EchoClueCombinator)),
        Box::new(Spaces::new(None, 'g')),
    ]))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    problem_to_url(combinator(), "echo", problem.clone())
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["echo"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    fn problem_for_tests() -> Problem {
        // 5x5 grid whose intended solution is:
        //   B W W W W
        //   B B B W W
        //   W W B W W
        //   W W B B B
        //   W W W W B
        // Every white cell carries the set of distances to the nearest black
        // cell in the four directions.
        vec![
            vec![None, Some(vec![1]), Some(vec![1, 2]), Some(vec![3]), Some(vec![3, 4])],
            vec![None, None, None, Some(vec![1, 2]), Some(vec![2])],
            vec![Some(vec![1, 2]), Some(vec![1]), None, Some(vec![1]), Some(vec![1, 2])],
            vec![Some(vec![2]), Some(vec![1, 2]), None, None, None],
            vec![Some(vec![3, 4]), Some(vec![3]), Some(vec![1, 2]), Some(vec![1]), None],
        ]
    }

    #[test]
    fn test_echo_problem() {
        let problem = problem_for_tests();
        let ans = solve_echo(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        let expected = crate::util::tests::to_option_bool_2d([
            [1, 0, 0, 0, 0],
            [1, 1, 1, 0, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 1, 1, 1],
            [0, 0, 0, 0, 1],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_echo_question_marks() {
        // Clue "?" alone: the set is {x} for some natural x, so a black cell
        // must appear at distance x in one direction, and every direction
        // with a black cell must be at that same distance.
        let problem: Problem = vec![vec![Some(vec![-1]), None], vec![None, None]];
        let ans = solve_echo(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        // The three valid solutions are {(0,1)}, {(1,0)} and
        // {(0,1),(1,1)}; only the clue cell is forced to stay white.
        assert_eq!(ans[0][0], Some(false));
        assert_eq!(ans[0][1], None);
        assert_eq!(ans[1][0], None);
        assert_eq!(ans[1][1], None);

        // Clue "?" at the corner of a 3x3 grid: a black cell at distance 1 or
        // 2 realizes the wildcard, so the puzzle is solvable and the clue
        // cell itself is always white.
        let problem: Problem = vec![
            vec![Some(vec![-1]), None, None],
            vec![None, None, None],
            vec![None, None, None],
        ];
        let ans = solve_echo(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();
        assert_eq!(ans[0][0], Some(false));

        // Clue "?" with no black cell in any direction is impossible.
        let problem: Problem = vec![vec![Some(vec![-1])]];
        assert!(solve_echo(&problem).is_none());

        // Two "?" in the same cell are two distinct numbers; around the
        // center of a 3x3 grid every ray has length 1, so two distinct
        // distances cannot both be realized.
        let problem: Problem = vec![
            vec![None, None, None],
            vec![None, Some(vec![-1, -1]), None],
            vec![None, None, None],
        ];
        assert!(solve_echo(&problem).is_none());

        // "2,?" around the center of a 3x3 grid: the printed number 2 must
        // be realized, but no ray has length 2.
        let problem: Problem = vec![
            vec![None, None, None],
            vec![None, Some(vec![2, -1]), None],
            vec![None, None, None],
        ];
        assert!(solve_echo(&problem).is_none());
    }

    #[test]
    fn test_echo_serializer() {
        let problem = problem_for_tests();
        let url = "https://puzz.link/p?echo/5/5/g1012030340i1202012010g1012020120i3403012010g";
        util::tests::serializer_test(problem, url, serialize_problem, deserialize_problem);
    }
}
