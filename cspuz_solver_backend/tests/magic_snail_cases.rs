use cspuz_solver_backend::solve_problem_json_from_bytes;

const MAGIC_SNAIL_SAMPLE: &[u8] = b"https://puzz.link/p?magic-snail/3/3/2/112221112221";
const MAGIC_SNAIL_PZV_SAMPLE: &[u8] = b"http://pzv.jp/p.html?magic-snail/3/3/2/112221112221";

#[test]
fn magic_snail_sample_is_solvable() {
    let response = solve_problem_json_from_bytes(MAGIC_SNAIL_SAMPLE);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
}

#[test]
fn magic_snail_pzv_url_is_solvable() {
    let response = solve_problem_json_from_bytes(MAGIC_SNAIL_PZV_SAMPLE);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
}

#[test]
fn magic_snail_outputs_numbers_and_blanks() {
    let response = solve_problem_json_from_bytes(MAGIC_SNAIL_SAMPLE);
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
