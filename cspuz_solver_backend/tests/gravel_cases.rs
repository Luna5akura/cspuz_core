use cspuz_solver_backend::solve_problem_json_from_bytes;

const USER_GRAVEL_URL: &[u8] = b"https://puzz.link/p?gravel/7/7/06000000060000000j2zi3z";
const NON_UNIQUE_GRAVEL_URL: &[u8] = b"https://puzz.link/p?gravel/7/7/00000000000000000i2zzk";
const NUMBERED_CLUE_GRAVEL_URL: &[u8] = b"https://puzz.link/p?gravel/7/7/00000000000000000i2q3s4y";

#[test]
fn gravel_one_cell_is_solvable() {
    let response = solve_problem_json_from_bytes(b"https://puzz.link/p?gravel/1/1/91");
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
    assert_eq!(response["description"]["isUnique"].as_bool(), Some(true));

    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();
    assert!(data.iter().any(|item| {
        item["color"].as_str() == Some("black") && item["item"].as_str() == Some("circle")
    }));
    assert!(!data.iter().any(|item| {
        item["color"].as_str() == Some("green")
            && (item["item"].as_str() == Some("dot") || item["item"].as_str() == Some("block"))
    }));
}

#[test]
fn gravel_user_case_does_not_duplicate_fixed_circle_cells() {
    let response = solve_problem_json_from_bytes(USER_GRAVEL_URL);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));

    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();
    let solver_cells = data
        .iter()
        .filter(|item| {
            item["color"].as_str() == Some("green")
                && (item["item"].as_str() == Some("dot") || item["item"].as_str() == Some("block"))
        })
        .count();
    assert_eq!(solver_cells, 47);

    for (y, x) in [(1, 9), (9, 1)] {
        assert!(!data.iter().any(|item| {
            item["y"].as_usize() == Some(y)
                && item["x"].as_usize() == Some(x)
                && item["color"].as_str() == Some("green")
                && item["item"].as_str() == Some("block")
        }));
    }
}

#[test]
fn gravel_non_unique_case_does_not_emit_a_full_first_solution() {
    let response = solve_problem_json_from_bytes(NON_UNIQUE_GRAVEL_URL);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
    assert_eq!(response["description"]["isUnique"].as_bool(), Some(false));

    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();
    let solver_cells = data
        .iter()
        .filter(|item| {
            item["color"].as_str() == Some("green")
                && (item["item"].as_str() == Some("dot") || item["item"].as_str() == Some("block"))
        })
        .count();

    assert!(solver_cells < 49);
}

#[test]
fn gravel_solver_overlay_does_not_cover_numbered_clues() {
    let response = solve_problem_json_from_bytes(NUMBERED_CLUE_GRAVEL_URL);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));

    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();
    for (y, x) in [(1, 7), (5, 3), (9, 3)] {
        assert!(!data.iter().any(|item| {
            item["y"].as_usize() == Some(y)
                && item["x"].as_usize() == Some(x)
                && item["color"].as_str() == Some("green")
                && (item["item"].as_str() == Some("dot") || item["item"].as_str() == Some("block"))
        }));
    }
}
