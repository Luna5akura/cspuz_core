use cspuz_solver_backend::solve_problem_json_from_bytes;

const MAGIC_SNAIL_SAMPLE: &[u8] = b"https://puzz.link/p?magic-snail/3/3/2/112221112221";
const MAGIC_SNAIL_PZV_SAMPLE: &[u8] = b"http://pzv.jp/p.html?magic-snail/3/3/2/112221112221";
const MAGIC_SNAIL_FIXED_BLANK_SAMPLE: &[u8] =
    b"https://puzz.link/p?magic-snail/3/3/2/112221112221.n";
const MAGIC_SNAIL_USER_SAMPLE: &[u8] =
    b"http://localhost:8080/p.html?magic-snail/8/8/4/zri2m4x3113x4m3i";

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

#[test]
fn magic_snail_outputs_fixed_blank_clues_as_black_crosses() {
    let response = solve_problem_json_from_bytes(MAGIC_SNAIL_FIXED_BLANK_SAMPLE);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));

    let data = response["description"]["data"]
        .members()
        .collect::<Vec<_>>();
    assert!(data.iter().any(|item| {
        item["y"].as_usize() == Some(1)
            && item["x"].as_usize() == Some(1)
            && item["color"].as_str() == Some("black")
            && item["item"].as_str() == Some("cross")
    }));
}

#[test]
fn magic_snail_user_sample_is_solvable() {
    let response = solve_problem_json_from_bytes(MAGIC_SNAIL_USER_SAMPLE);
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
}
