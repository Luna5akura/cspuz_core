use cspuz_solver_backend::solve_problem_json_from_bytes;

const MAGIC_SUMMER_SAMPLE: &[u8] = b"https://puzz.link/p?magic-summer/3/3/2/c3c-153-15c3c-153-15";

#[test]
fn magic_summer_sample_is_solvable() {
    let response = solve_problem_json_from_bytes(MAGIC_SUMMER_SAMPLE);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
}

#[test]
fn magic_summer_outputs_numbers_and_blanks() {
    let response = solve_problem_json_from_bytes(MAGIC_SUMMER_SAMPLE);
    let response = json::parse(&response).unwrap();
    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();

    let mut nums = 0;
    let mut crosses = 0;
    for item in data {
        if item["item"].as_str() == Some("cross") {
            crosses += 1;
        } else if item["item"]["kind"].as_str() == Some("text") {
            nums += 1;
        }
    }

    assert_eq!(nums, 6);
    assert_eq!(crosses, 3);
}
