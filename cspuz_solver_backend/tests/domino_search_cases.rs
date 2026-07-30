use cspuz_solver_backend::solve_problem_json_from_bytes;

const USER_URL: &[u8] =
    b"https://puzz.link/p?domino-search/8/8/14541112340g634523g0g1605511g223566g546326i3432000040261352456";

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
    let data = response["description"]["data"].members().collect::<Vec<_>>();

    let mut crosses = 0;
    for item in data {
        if item["item"].as_str() == Some("cross") {
            crosses += 1;
        }
        assert_ne!(
            (item["color"].as_str(), item["item"].as_str()),
            (Some("#cccccc"), Some("wall"))
        );
    }
    assert_eq!(crosses, 28);
}
