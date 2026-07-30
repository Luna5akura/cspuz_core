use cspuz_solver_backend::solve_problem_json_from_bytes;

#[test]
fn gravel_one_cell_is_solvable() {
    let response = solve_problem_json_from_bytes(b"https://puzz.link/p?gravel/1/1/91");
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));

    let data = response["description"]["data"].members().collect::<Vec<_>>();
    assert!(data.iter().any(|item| {
        item["color"].as_str() == Some("green") && item["item"].as_str() == Some("dot")
    }));
}
