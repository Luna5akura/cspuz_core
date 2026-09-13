use cspuz_solver_backend::solve_problem_json_from_bytes;

const PUZZLE_22: &str = "https://puzz.link/p?sky-neighbor/9/9/..........1.............................2.............................3........../.G..GG.GG;GGGG.G...;.G.G...GG;.G..G..G.;.GG....G.;.G.G....G;.G.GGGG.G;GGG....GG;.G.G.GGGG/212221313/212223121/213211232/221223121/010001111/010001111/011100010/001001111";

// In the printed puzzle the outside ring is initially blank; its values are
// derived from visibility.  Keep this variant here to ensure the solver does
// not accidentally rely on the answer values being supplied as givens.
const PUZZLE_22_BLANK_OUTER: &str = "https://puzz.link/p?sky-neighbor/9/9/..........1.............................2.............................3........../.G..GG.GG;GGGG.G...;.G.G...GG;.G..G..G.;.GG....G.;.G.G....G;.G.GGGG.G;GGG....GG;.G.G.GGGG/........./........./........./........./010001111/010001111/011100010/001001111";

#[test]
fn sky_neighbor_published_example_is_solvable() {
    let response = solve_problem_json_from_bytes(PUZZLE_22.as_bytes());
    let response = json::parse(&response).unwrap();

    assert_eq!(response["status"].as_str(), Some("ok"));
    let board = &response["description"];
    assert_eq!(board["defaultStyle"].as_str(), Some("outer_grid"));
    assert_eq!(board["height"].as_usize(), Some(11));
    assert_eq!(board["width"].as_usize(), Some(11));

    let data = board["data"].members().collect::<Vec<_>>();
    assert!(data.iter().any(|entry| {
        entry["color"].as_str() == Some("black")
            && entry["item"].as_str() == Some("square")
    }));
    assert!(data.iter().any(|entry| {
        entry["color"].as_str() == Some("black")
            && entry["item"]["kind"].as_str() == Some("text")
    }));
    assert!(data.iter().any(|entry| {
        entry["color"].as_str() == Some("green")
            && entry["item"]["kind"].as_str() == Some("text")
    }));

    // The backend uses the 11x11 outer-grid coordinate space.  Every emitted
    // item must be inside that space and answer pieces lie on odd/odd cells.
    for entry in &data {
        let x = entry["x"].as_usize().expect("x coordinate");
        let y = entry["y"].as_usize().expect("y coordinate");
        assert!(x <= 21 && y <= 21);
        if entry["item"].as_str().is_none() {
            assert_eq!(x % 2, 1);
            assert_eq!(y % 2, 1);
        }
    }
}

#[test]
fn sky_neighbor_aliases_dispatch_to_the_same_solver() {
    for alias in [
        "skyneighbor",
        "skyneighbors",
        "sky-neighbor",
        "sky-neighbors",
        "skyneighbour",
        "skyneighbours",
        "sky-neighbour",
        "sky-neighbours",
    ] {
        let url = PUZZLE_22.replacen("sky-neighbor", alias, 1);
        let response = solve_problem_json_from_bytes(url.as_bytes());
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{alias}");
        assert_eq!(response["description"]["height"].as_usize(), Some(11));
        assert_eq!(response["description"]["width"].as_usize(), Some(11));
    }
}

#[test]
fn sky_neighbor_solves_when_outside_values_are_blank() {
    let response = solve_problem_json_from_bytes(PUZZLE_22_BLANK_OUTER.as_bytes());
    let response = json::parse(&response).unwrap();
    assert_eq!(response["status"].as_str(), Some("ok"));
}
