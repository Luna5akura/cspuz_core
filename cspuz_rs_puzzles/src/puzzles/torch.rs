use cspuz_rs::graph;
use cspuz_rs::serializer::{
    problem_to_url_with_context, url_to_problem, Choice, Combinator, Context, ContextBasedGrid,
    Dict, Number16, Optionalize, Size, Spaces,
};
use cspuz_rs::solver::{any, count_true, Solver};

/// Torch:
/// 黒マスを全体でひとつながりに、2x2 を作らず、輪っかを作らずに配置する。
/// 数字の入った黒マスは、そのマスから最も近い「端点」(黒マスに1つだけ
/// 隣接する黒マス)までの黒マス上を歩いた距離を表す。
///
/// 問題の表現: 各セルは
///   None     = 白マス
///   Some(-2) = 数字なしの黒マス(問題で確定)
///   Some(n)  = 数字 n の黒マス(問題で確定)
pub type Problem = Vec<Vec<Option<i32>>>;

pub fn solve_torch(problem: &Problem) -> Option<Vec<Vec<Option<bool>>>> {
    let (h, w) = crate::util::infer_shape(problem);

    let mut solver = Solver::new();
    let is_black = &solver.bool_var_2d((h, w));
    solver.add_answer_key_bool(is_black);

    // 問題で確定した黒マス
    for y in 0..h {
        for x in 0..w {
            if problem[y][x].is_some() {
                solver.add_expr(is_black.at((y, x)));
            }
        }
    }

    // 黒マスは全体でひとつながり
    graph::active_vertices_connected_2d(&mut solver, is_black);

    // 2x2 の黒マスを作らない
    if h >= 2 && w >= 2 {
        solver.add_expr(!is_black.conv2d_and((2, 2)));
    }

    // 輪っかを作らない: 連結な黒マスの辺の数 = 黒マスの数 - 1 (木)
    let mut edge_list = vec![];
    for y in 0..h {
        for x in 0..w {
            if x + 1 < w {
                edge_list.push(is_black.at((y, x)) & is_black.at((y, x + 1)));
            }
            if y + 1 < h {
                edge_list.push(is_black.at((y, x)) & is_black.at((y + 1, x)));
            }
        }
    }
    let edge_count = count_true(edge_list);
    solver.add_expr(edge_count.eq(is_black.count_true() - 1));

    // 端点: 黒マスに1つだけ隣接する黒マス
    let is_leaf = &solver.bool_var_2d((h, w));
    for y in 0..h {
        for x in 0..w {
            let neighbors = vec![
                is_black.at_offset((y, x), (-1, 0), false),
                is_black.at_offset((y, x), (1, 0), false),
                is_black.at_offset((y, x), (0, -1), false),
                is_black.at_offset((y, x), (0, 1), false),
            ];
            solver.add_expr(
                is_leaf.at((y, x))
                    .iff(is_black.at((y, x)) & count_true(neighbors).eq(1)),
            );
        }
    }

    // 各セルから最も近い端点までの距離を多層伝播で計算する
    //   layer[0] = 端点そのもの
    //   layer[d+1] = 黒マスで、隣に layer[d] のマスがある (または端点)
    let max_n = problem
        .iter()
        .flatten()
        .filter_map(|&c| c)
        .filter(|&n| n >= 0)
        .max()
        .unwrap_or(0) as usize;

    let mut layers = vec![is_leaf.clone()];
    for d in 1..=max_n {
        let layer = &solver.bool_var_2d((h, w));
        let prev = &layers[d - 1];
        for y in 0..h {
            for x in 0..w {
                let neighbor = any(vec![
                    prev.at_offset((y, x), (-1, 0), false),
                    prev.at_offset((y, x), (1, 0), false),
                    prev.at_offset((y, x), (0, -1), false),
                    prev.at_offset((y, x), (0, 1), false),
                ]);
                solver.add_expr(
                    layer
                        .at((y, x))
                        .iff(is_black.at((y, x)) & (is_leaf.at((y, x)) | neighbor)),
                );
            }
        }
        layers.push(layer.clone());
    }

    // 数字の制約: 最も近い端点までの距離がちょうど n
    for y in 0..h {
        for x in 0..w {
            if let Some(n) = problem[y][x] {
                if n < 0 {
                    continue;
                }
                let n = n as usize;
                if n == 0 {
                    solver.add_expr(is_leaf.at((y, x)));
                } else {
                    solver.add_expr(layers[n].at((y, x)) & !layers[n - 1].at((y, x)));
                }
            }
        }
    }

    solver.irrefutable_facts().map(|f| f.get(is_black))
}

fn combinator() -> impl Combinator<Problem> {
    Size::new(ContextBasedGrid::new(Choice::new(vec![
        Box::new(Optionalize::new(Number16)),
        Box::new(Dict::new(Some(-2), ".")),
        Box::new(Spaces::new(None, 'g')),
    ])))
}

pub fn serialize_problem(problem: &Problem) -> Option<String> {
    let (h, w) = crate::util::infer_shape(problem);
    problem_to_url_with_context(combinator(), "torch", problem.clone(), &Context::sized(h, w))
}

pub fn deserialize_problem(url: &str) -> Option<Problem> {
    url_to_problem(combinator(), &["torch"], url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_for_tests() -> Problem {
        // 5x5: 数字1の黒マス(1,2)と数字0の黒マス(3,2)が与えられ、
        // 正解は (1,1),(1,2),(1,3),(2,2),(3,2) の木
        let mut ret = vec![vec![None; 5]; 5];
        ret[1][2] = Some(1);
        ret[3][2] = Some(0);
        ret
    }

    #[test]
    fn test_torch_problem() {
        let problem = problem_for_tests();
        let ans = solve_torch(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();

        // 与えられた黒マスは確定し、それ以外にも解が存在する
        assert_eq!(ans[1][2], Some(true));
        assert_eq!(ans[3][2], Some(true));
    }

    #[test]
    fn test_torch_unique_problem() {
        // 十字の木: 中央の1以外はすべて数字0の端点
        let mut problem = vec![vec![None; 5]; 5];
        problem[0][2] = Some(0);
        problem[1][1] = Some(0);
        problem[1][2] = Some(1);
        problem[1][3] = Some(0);
        problem[2][2] = Some(0);
        let ans = solve_torch(&problem);
        assert!(ans.is_some());
        let ans = ans.unwrap();

        let expected = crate::util::tests::to_option_bool_2d([
            [0, 0, 1, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 1, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
        ]);
        assert_eq!(ans, expected);
    }

    #[test]
    fn test_torch_serializer() {
        let problem = problem_for_tests();
        let url = serialize_problem(&problem).expect("serialize");
        assert!(url.contains("torch/"));
        let restored = deserialize_problem(&url).expect("deserialize");
        assert_eq!(restored, problem);
    }

    #[test]
    fn test_torch_rejects_cycle() {
        // 3x3 の外周リング: 連結・2x2なしだが輪っかになるため解なし
        let mut problem = vec![vec![None; 3]; 3];
        for y in 0..3 {
            for x in 0..3 {
                if y == 0 || y == 2 || x == 0 || x == 2 {
                    problem[y][x] = Some(-2);
                }
            }
        }
        assert!(solve_torch(&problem).is_none());
    }

    #[test]
    fn test_torch_unreachable_distance_unsat() {
        // 1x3 では端点までの距離は最大1なので、数字5は満たせない
        let problem = vec![vec![Some(5), None, None]];
        assert!(solve_torch(&problem).is_none());
    }
}
