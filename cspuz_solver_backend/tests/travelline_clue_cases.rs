use cspuz_solver_backend::solve_custom_travelline_json_from_bytes;

#[test]
fn travelline_clue_examples_from_urls_return_expected_status() {
    let cases = json::parse(include_str!("fixtures/travelline_clue_cases.json"))
        .expect("travelline clue fixture should parse");

    for case in cases.members() {
        let name = case["name"].as_str().unwrap_or("<unnamed>");
        let source_url = case["source_url"].as_str().unwrap_or("<missing url>");
        let expected_status = case["expected_status"].as_str().unwrap_or("ok");
        let payload = case["payload"].dump();
        let actual = solve_custom_travelline_json_from_bytes(payload.as_bytes());
        let response = json::parse(&actual)
            .unwrap_or_else(|_| panic!("response should parse for {name}: {actual}"));

        assert_eq!(
            response["status"].as_str(),
            Some(expected_status),
            "travelline clue case {name} from {source_url} returned an unexpected status: {actual}"
        );

        if expected_status == "ok" {
            assert_eq!(
                response["description"]["kind"].as_str(),
                Some("grid"),
                "travelline clue case {name} from {source_url} returned an invalid board: {actual}"
            );
            if let Some(expected_is_unique) = case["expected_is_unique"].as_bool() {
                assert_eq!(
                    response["description"]["isUnique"].as_bool(),
                    Some(expected_is_unique),
                    "travelline clue case {name} from {source_url} returned unexpected uniqueness: {actual}"
                );
            }
        } else if let Some(expected_description) = case["expected_description"].as_str() {
            assert_eq!(
                response["description"].as_str(),
                Some(expected_description),
                "travelline clue case {name} from {source_url} returned an unexpected error: {actual}"
            );
        }
    }
}
