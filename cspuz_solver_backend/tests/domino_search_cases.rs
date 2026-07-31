use cspuz_solver_backend::solve_problem_json_from_bytes;

const USER_URL: &[u8] =
    b"https://puzz.link/p?domino-search/8/8/14541112340g634523g0g1605511g223566g546326i3432000040261352456";
const USER_URL_WITH_BLOCKS: &[u8] =
    b"https://puzz.link/p?domino-search/8/8/16223121140h40013g5105065g35g4641426g66540h6262221355333440503";

#[test]
fn domino_search_with_empty_cells_is_solvable() {
    let response = solve_problem_json_from_bytes(USER_URL);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
}

#[test]
fn domino_search_does_not_emit_placeholder_walls_on_open_edges() {
    let response = solve_problem_json_from_bytes(USER_URL);
    let response = json::parse(&response).unwrap();
    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();

    for item in data {
        assert_ne!(
            (item["color"].as_str(), item["item"].as_str()),
            (Some("#cccccc"), Some("wall"))
        );
        assert_ne!(
            (item["color"].as_str(), item["item"].as_str()),
            (Some("green"), Some("cross"))
        );
    }
}

#[test]
fn domino_search_solver_outputs_only_solution_borders() {
    let response = solve_problem_json_from_bytes(USER_URL_WITH_BLOCKS);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));

    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();
    let block_cells = data
        .iter()
        .filter(|item| {
            item["color"].as_str() == Some("black") && item["item"].as_str() == Some("block")
        })
        .map(|item| (item["y"].as_usize().unwrap(), item["x"].as_usize().unwrap()))
        .collect::<Vec<_>>();

    let mut walls = 0;
    for item in data {
        if item["color"].as_str() != Some("green") {
            continue;
        }
        assert_eq!(item["item"].as_str(), Some("boldWall"));
        walls += 1;

        let y = item["y"].as_usize().unwrap();
        let x = item["x"].as_usize().unwrap();
        let adjacent = if y % 2 == 0 {
            [(y - 1, x), (y + 1, x)]
        } else {
            [(y, x - 1), (y, x + 1)]
        };
        assert!(!block_cells.iter().any(|cell| adjacent.contains(cell)));
    }
    assert_eq!(walls, 56);
}
