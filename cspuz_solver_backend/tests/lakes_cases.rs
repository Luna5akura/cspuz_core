use cspuz_solver_backend::solve_problem_json_from_bytes;

#[test]
fn lakes_sample_is_solvable() {
    let response = solve_problem_json_from_bytes(b"https://puzz.link/p?lakes/3/3/2g2l");
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
}
