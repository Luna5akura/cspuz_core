#![allow(static_mut_refs)] // TODO: remove this

extern crate cspuz_rs;

pub mod board;
mod custom_travelline;
mod forced_lines;
mod puzzle;
mod uniqueness;

use board::Board;
use cspuz_rs::serializer::{get_kudamono_url_info_detailed, url_to_puzzle_kind};
pub use puzzle::{list_penpa_edit_puzzles, list_puzzles_for_enumerate, list_puzzles_for_solve};

fn parse_penpa_edit_special_url(url: &str) -> Option<(&str, &str)> {
    let separator = url.find("!")?;
    let kind = &url[..separator];
    let url = &url[separator + 1..];

    if !(url.starts_with("https://opt-pan.github.io/penpa-edit/")
        || url.starts_with("penpa-edit-predecoded:"))
    {
        return None;
    }

    Some((kind, url))
}

fn decode_and_solve(url: &[u8]) -> Result<Board, &'static str> {
    let url = std::str::from_utf8(url).map_err(|_| "failed to decode URL as UTF-8")?;

    if let Some(puzzle_kind) = url_to_puzzle_kind(url) {
        return puzzle::dispatch_puzz_link(&puzzle_kind, url).unwrap_or(Err("unknown puzzle type"));
    }

    if let Some(puzzle_info) = get_kudamono_url_info_detailed(url) {
        let puzzle_kind = *puzzle_info.get("G").unwrap_or(&"");
        let puzzle_variant = *puzzle_info.get("V").unwrap_or(&"");

        return puzzle::dispatch_kudamono(puzzle_kind, puzzle_variant, url)
            .unwrap_or(Err("unknown puzzle type"));
    }

    if let Some((kind, url)) = parse_penpa_edit_special_url(url) {
        return puzzle::dispatch_penpa_edit(kind, url).unwrap_or(Err("unknown puzzle type"));
    }

    Err("URL cannot be parsed")
}

fn decode_and_enumerate(
    url: &[u8],
    num_max_answers: usize,
) -> Result<(Board, Vec<Board>), &'static str> {
    let url = std::str::from_utf8(url).map_err(|_| "failed to decode URL as UTF-8")?;

    let puzzle_kind = url_to_puzzle_kind(url).ok_or("puzzle type not detected")?;

    puzzle::dispatch_puzz_link_enumerate(&puzzle_kind, url, num_max_answers)
        .unwrap_or(Err("unknown puzzle type"))
}

fn solve_custom_travelline_payload(payload: &[u8]) -> Result<Board, &'static str> {
    let payload =
        std::str::from_utf8(payload).map_err(|_| "failed to decode travelline payload as UTF-8")?;
    let problem = custom_travelline::deserialize_problem(payload)?;
    custom_travelline::solve(&problem)
}

fn decode_and_solve_with_forced_lines(payload: &[u8]) -> Result<Board, &'static str> {
    let payload = forced_lines::deserialize_payload(payload)?;
    let puzzle_kind = url_to_puzzle_kind(&payload.url).ok_or("puzzle type not detected")?;
    puzzle::dispatch_puzz_link_with_forced_lines(&puzzle_kind, &payload.url, &payload.forced_lines)
        .unwrap_or(Err(
            "forced line constraints are not supported for this puzzle",
        ))
}

pub fn solve_problem_json_from_bytes(url: &[u8]) -> String {
    let result = decode_and_solve(url);
    match result {
        Ok(board) => {
            format!("{{\"status\":\"ok\",\"description\":{}}}", board.to_json())
        }
        Err(err) => {
            format!("{{\"status\":\"error\",\"description\":\"{}\"}}", err)
        }
    }
}

pub fn solve_problem_with_forced_lines_json_from_bytes(payload: &[u8]) -> String {
    let result = decode_and_solve_with_forced_lines(payload);
    match result {
        Ok(board) => {
            format!("{{\"status\":\"ok\",\"description\":{}}}", board.to_json())
        }
        Err(err) => {
            format!("{{\"status\":\"error\",\"description\":\"{}\"}}", err)
        }
    }
}

pub fn enumerate_answers_json_from_bytes(url: &[u8], num_max_answers: usize) -> String {
    let result = decode_and_enumerate(url, num_max_answers);
    match result {
        Ok((common, per_answer)) => {
            format!(
                "{{\"status\":\"ok\",\"description\":{{\"common\":{},\"answers\":[{}]}}}}",
                common.to_json(),
                per_answer
                    .iter()
                    .map(|x| x.to_json())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Err(err) => {
            format!("{{\"status\":\"error\",\"description\":\"{}\"}}", err)
        }
    }
}

pub fn solve_custom_travelline_json_from_bytes(payload: &[u8]) -> String {
    let result = solve_custom_travelline_payload(payload);
    match result {
        Ok(board) => {
            format!("{{\"status\":\"ok\",\"description\":{}}}", board.to_json())
        }
        Err(err) => {
            format!("{{\"status\":\"error\",\"description\":\"{}\"}}", err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{solve_problem_json_from_bytes, solve_problem_with_forced_lines_json_from_bytes};

    fn forced_grid(
        height: usize,
        width: usize,
        forced: Option<(usize, usize, i32)>,
    ) -> json::JsonValue {
        let mut grid = json::JsonValue::new_array();
        for y in 0..height {
            let mut row = json::JsonValue::new_array();
            for x in 0..width {
                let value = match forced {
                    Some((forced_y, forced_x, value)) if forced_y == y && forced_x == x => value,
                    _ => -1,
                };
                row.push(value).unwrap();
            }
            grid.push(row).unwrap();
        }
        grid
    }

    fn simpleloop_forced_payload(value: i32) -> String {
        json::object! {
            url: "https://puzz.link/p?simpleloop/8/7/200200a42000",
            forcedH: forced_grid(7, 7, Some((3, 1, value))),
            forcedV: forced_grid(6, 8, None),
        }
        .dump()
    }

    #[test]
    fn solve_problem_with_forced_lines_respects_simpleloop_constraints() {
        let ok_response = solve_problem_with_forced_lines_json_from_bytes(
            simpleloop_forced_payload(1).as_bytes(),
        );
        let ok_response = json::parse(&ok_response).unwrap();
        assert_eq!(ok_response["status"].as_str(), Some("ok"));

        let no_answer_response = solve_problem_with_forced_lines_json_from_bytes(
            simpleloop_forced_payload(0).as_bytes(),
        );
        let no_answer_response = json::parse(&no_answer_response).unwrap();
        assert_eq!(no_answer_response["status"].as_str(), Some("error"));
        assert_eq!(
            no_answer_response["description"].as_str(),
            Some("no answer")
        );
    }

    #[test]
    fn solve_problem_dispatches_domino_search() {
        let response =
            solve_problem_json_from_bytes(b"https://puzz.link/p?domino-search/4/3/000111021222");
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
    }

    #[test]
    fn solve_problem_dispatches_slovak_sums() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?slovak-sums/3/3/eyJudW1iZXJzIjpbMSwyXSwiY2VsbHMiOltbbnVsbCxudWxsLHsic3VtIjozLCJjb3VudCI6Mn1dLFtudWxsLHsic3VtIjo2LCJjb3VudCI6NH0sbnVsbF0sW3sic3VtIjozLCJjb3VudCI6Mn0sbnVsbCxudWxsXV19",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
    }

    #[test]
    fn solve_problem_dispatches_native_slovak_sums() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?slovak-sums/5/5/3/1n-28i-26g0m-1cg",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
    }

    #[test]
    fn solve_problem_dispatches_native_slovak_sums_8x8() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?slovak-sums/8/8/4/g-16o-11o-30o-35-3ao-4fo-3bo-16g",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
    }

    #[test]
    fn solve_problem_dispatches_kakuro_without_synthetic_frame() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?kakuro/6/5/Dclh4t9fl3-p-gl-alJeC3BgG",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));

        let board = &response["description"];
        // The URL's clue matrix contains a synthetic top row/left column;
        // those are not playable cells in pzpr and must not be exposed in
        // the solver description.
        assert_eq!(board["height"].as_usize(), Some(5));
        assert_eq!(board["width"].as_usize(), Some(6));

        let data = board["data"].members().collect::<Vec<_>>();
        assert!(!data.is_empty());
        assert!(data
            .iter()
            .all(|entry| entry["color"].as_str() == Some("green")));
        assert!(data.iter().any(|entry| {
            entry["x"].as_usize() == Some(3)
                && entry["y"].as_usize() == Some(1)
                && entry["item"]["data"].as_str() == Some("3")
        }));
    }

    #[test]
    fn solve_problem_dispatches_neighbors() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?neighbor/9/9/............3.......2.......1...3.......2.......1...3.......2.......1............/111111101001110001011110001110111111110000101101111001100001111110101001000110000",
        );
        let response = json::parse(&response).unwrap();

        assert_eq!(response["status"].as_str(), Some("ok"));
        let board = &response["description"];
        assert_eq!(board["height"].as_usize(), Some(9));
        assert_eq!(board["width"].as_usize(), Some(9));

        let data = board["data"].members().collect::<Vec<_>>();
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("black") && entry["item"].as_str() == Some("square")
        }));
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("black")
                && entry["item"]["kind"].as_str() == Some("text")
        }));
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("green")
                && entry["item"]["kind"].as_str() == Some("text")
        }));
    }

    #[test]
    fn solve_problem_dispatches_sky_neighbors() {
        // Sky-neighbors uses the published Round 4 format: a 9x9 inner
        // Neighbors grid followed by four visibility-count sides.  Keep this
        // at the dispatch boundary so aliases, URL decoding, and the
        // outer-grid board representation are covered together.
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?sky-neighbor/9/9/..........1.............................2.............................3........../.G..GG.GG;GGGG.G...;.G.G...GG;.G..G..G.;.GG....G.;.G.G....G;.G.GGGG.G;GGG....GG;.G.G.GGGG/212221313/212223121/213211232/221223121/010001111/010001111/011100010/001001111",
        );
        let response = json::parse(&response).unwrap();

        assert_eq!(response["status"].as_str(), Some("ok"));
        let board = &response["description"];
        assert_eq!(board["defaultStyle"].as_str(), Some("outer_grid"));
        assert_eq!(board["height"].as_usize(), Some(11));
        assert_eq!(board["width"].as_usize(), Some(11));

        // Both inner answers and outside visibility counts are emitted as
        // solver-derived (green) numbers when they are not givens.
        let data = board["data"].members().collect::<Vec<_>>();
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("green")
                && entry["item"]["kind"].as_str() == Some("text")
        }));
    }

    #[test]
    fn solve_problem_dispatches_wolves_and_sheep_fences() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?wolvesandsheepfences/5/5/2c5a2a5c2a136a3c6a3",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
        assert_eq!(response["description"]["height"].as_usize(), Some(5));
        assert_eq!(response["description"]["width"].as_usize(), Some(5));
    }

    #[test]
    fn solve_problem_dispatches_fourwindswithparks() {
        let response =
            solve_problem_json_from_bytes(b"https://puzz.link/p?fourwindswithparks/3/3/0j2g1g");
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
    }

    #[test]
    fn solve_problem_dispatches_japanese_arrows() {
        let response =
            solve_problem_json_from_bytes(b"https://puzz.link/p?japanese_arrows/3/3/42h");
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
    }

    #[test]
    fn solve_problem_dispatches_shape_minesweeper() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?shapeminesweeper/4/4/................//t",
        );
        let response = json::parse(&response).unwrap();
        // The empty-clue board with the standard bank is intentionally not
        // necessarily solvable; this assertion verifies that the puzzle is
        // decoded and dispatched instead of returning an unknown-type error.
        assert_ne!(
            response["description"].as_str(),
            Some("unknown puzzle type")
        );
    }

    #[test]
    fn solve_problem_does_not_overlay_slovak_sums_clues() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?slovak-sums/8/8/4/g-16o-11o-30o-35-3ao-4fo-3bo-16g",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
        assert!(!response.to_string().contains("#cccccc"));
    }
}
