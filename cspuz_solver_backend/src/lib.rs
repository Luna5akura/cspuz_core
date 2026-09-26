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
    fn solve_problem_lostspeech_shows_only_certain_facts() {
        // 4x4ツイン盤: 盤面1は2x2正方形×2 (一意)、盤面2は青=3連トロミノ
        // (縦/横の2通り) + 赤=Tテトリミノ。
        // 盤面2の青の置き方が2通りあるため共同の解は非一意。
        // ソルバー表示は「どの解でも成り立つ」内容のみを含む:
        //  - 盤面1の2枚の正方形 (8マス) と盤面2の赤T (4マス) は確定
        //  - 盤面2の青は起点(0,0)のみ確定
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?lostspeech/4/4/6222222g2g22g2270000/4/22u/22u/13s/23eg&variant=1",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
        assert_eq!(response["description"]["isUnique"], false);

        let data = response["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert_eq!(
            data.iter()
                .filter(|e| e["item"].as_str() == Some("fill"))
                .count(),
            13,
            "8 certain cells on board 1 + 5 on board 2: {}",
            response
        );
        // 盤面2の青の不確定なマス (1,0) は塗られない
        assert!(!data.iter().any(|e| {
            e["item"].as_str() == Some("fill")
                && e["x"].as_usize() == Some(23)
                && e["y"].as_usize() == Some(3)
        }));
    }

    #[test]
    fn solve_problem_lostspeech_accepts_pzpr_variant_double_slash() {
        // ブラウザでvariant有効時のURL形態 (v: 段の後ろにスラッシュが残る) も受理する
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?lostspeech//8/8/q122j1111i561g11j2222zk0000000000000/4/12o/22u/22e/22u&variant=1",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
        assert_eq!(response["description"]["isUnique"], true);
    }

    #[test]
    fn solve_problem_dispatches_battleships() {
        // 5x1: 2マス艦2隻 → 唯一解
        let response =
            solve_problem_json_from_bytes(b"https://puzz.link/p?battleships/5/1/lk/2/21o/21o");
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
        assert_eq!(response["description"]["isUnique"], true);
        let data = response["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert_eq!(
            data.iter()
                .filter(|e| e["item"].as_str() == Some("fill"))
                .count(),
            4,
            "both ships must be certain: {}",
            response
        );
    }

    #[test]
    fn solve_problem_dispatches_place_by_product() {
        // 4x4: 2x2と1x4のピース → 唯一解
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?placebyproduct/4/4/22401133000000/2/22u/14u",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
        assert_eq!(response["description"]["isUnique"], true);
        let data = response["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert_eq!(
            data.iter()
                .filter(|e| e["item"].as_str() == Some("fill"))
                .count(),
            8,
            "all shape cells must be certain: {}",
            response
        );
    }

    #[test]
    fn solve_problem_lostspeech_accepts_pzpr_variant_segment() {
        // pzprの「v:バリアントID」セグメントを含むURLも受理する
        // (variantチェックボックスを有効にするとURLに v: が入る)
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?lostspeech/v:/8/8/q122j1111i561g11j2222zk0000000000000/4/12o/22u/22e/22u&variant=1",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
        assert_eq!(response["description"]["isUnique"], true);
    }

    #[test]
    fn solve_problem_lostspeech_two_solutions_shown_per_board() {
        // 2x2 (61170): 各盤面に (縦,縦) と (横,横) の2解。
        // variant=0 では左盤面に1つ目、右盤面に2つ目の解が表示される。
        let without_variant = solve_problem_json_from_bytes(
            b"https://puzz.link/p?lostspeech/2/2/61170/4/12o/12o/12o/12o&variant=0",
        );
        let without_variant = json::parse(&without_variant).unwrap();
        assert_eq!(without_variant["status"].as_str(), Some("ok"));
        let data = without_variant["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        // 左右それぞれ4マス (青2+赤2) が具体的な解として塗られる
        assert_eq!(
            data.iter()
                .filter(|e| e["item"].as_str() == Some("fill"))
                .count(),
            8,
            "two concrete solutions must be shown: {}",
            without_variant
        );

        // variant=1 では共同の解の確定事実のみ (起点マスだけ)
        let with_variant = solve_problem_json_from_bytes(
            b"https://puzz.link/p?lostspeech/2/2/61170/4/12o/12o/12o/12o&variant=1",
        );
        let with_variant = json::parse(&with_variant).unwrap();
        assert_eq!(with_variant["status"].as_str(), Some("ok"));
        let data2 = with_variant["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert_eq!(
            data2.iter()
                .filter(|e| e["item"].as_str() == Some("fill"))
                .count(),
            4,
            "only the certain cells must be shown: {}",
            with_variant
        );
    }

    #[test]
    fn solve_problem_lostspeech_variant_rule_toggles_cross_containment() {
        // 例题3 (q122j): variant=1 で唯一解、variant=0 では跨盤包含なしで非一意
        let base = "https://puzz.link/p?lostspeech/8/8/q122j1111i561g11j2222zk0000000000000/4/12o/22u/22e/22u";
        let with_variant = solve_problem_json_from_bytes(format!("{}&variant=1", base).as_bytes());
        let with_variant = json::parse(&with_variant).unwrap();
        assert_eq!(with_variant["status"].as_str(), Some("ok"));
        assert_eq!(with_variant["description"]["isUnique"], true);

        let without_variant =
            solve_problem_json_from_bytes(format!("{}&variant=0", base).as_bytes());
        let without_variant = json::parse(&without_variant).unwrap();
        assert_eq!(without_variant["status"].as_str(), Some("ok"));
        assert_eq!(without_variant["description"]["isUnique"], false);
    }

    #[test]
    fn solve_problem_lostspeech_unique_shows_full_solution() {
        // 一意に解ける4x4ツイン盤: 盤面1は2x2正方形2枚、盤面2はTテトリミノ2枚。
        // ソルバー表示には両盤面の全形状 (計16マスの塗り) と境界線が含まれる。
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?lostspeech/4/4/622g222gh22g2270000/4/22u/22u/32t0/23eg&variant=1",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));
        assert_eq!(response["description"]["isUnique"], true);

        let data = response["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert_eq!(
            data.iter()
                .filter(|e| e["item"].as_str() == Some("fill"))
                .count(),
            16,
            "all 16 shape cells must be certain: {}",
            response
        );
        // 両盤面に境界線が描かれている
        assert!(data.iter().any(|e| {
            e["item"].as_str() == Some("wall") && e["x"].as_usize().unwrap() < 20
        }));
        assert!(data.iter().any(|e| {
            e["item"].as_str() == Some("wall") && e["x"].as_usize().unwrap() >= 20
        }));
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
    fn solve_problem_dispatches_consecutive_kakuro_with_bars() {
        // 2x2 board: row sums 4/6, column sums 3/7, bars between the two
        // consecutive pairs (a-c and b-d).  The unique answer is
        //   1 3
        //   2 4
        // which only holds when the white-bar constraints are enforced.
        let response =
            solve_problem_json_from_bytes(b"https://puzz.link/p?consecutivekakuro/2/2/n37461100");
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));

        let board = &response["description"];
        assert_eq!(board["height"].as_usize(), Some(2));
        assert_eq!(board["width"].as_usize(), Some(2));

        let data = board["data"].members().collect::<Vec<_>>();
        assert_eq!(data.len(), 4);
        assert!(data.iter().any(|entry| {
            entry["x"].as_usize() == Some(1)
                && entry["y"].as_usize() == Some(1)
                && entry["item"]["data"].as_str() == Some("1")
        }));
        assert!(data.iter().any(|entry| {
            entry["x"].as_usize() == Some(3)
                && entry["y"].as_usize() == Some(1)
                && entry["item"]["data"].as_str() == Some("3")
        }));
        assert!(data.iter().any(|entry| {
            entry["x"].as_usize() == Some(1)
                && entry["y"].as_usize() == Some(3)
                && entry["item"]["data"].as_str() == Some("2")
        }));
        assert!(data.iter().any(|entry| {
            entry["x"].as_usize() == Some(3)
                && entry["y"].as_usize() == Some(3)
                && entry["item"]["data"].as_str() == Some("4")
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
    fn solve_problem_dispatches_pills() {
        // WPF GP 2015 Round 7, puzzle 20.  The pill values appear in every
        // covered cell; uncovered cells stay empty.
        let problem = cspuz_rs_puzzles::puzzles::pills::PillsProblem {
            dots: vec![
                vec![4, 4, 4, 2, 2, 2, 2, 4, 4, 4],
                vec![4, 2, 4, 1, 1, 1, 1, 4, 2, 4],
                vec![4, 4, 4, 1, 0, 0, 1, 4, 4, 4],
                vec![2, 1, 1, 1, 0, 0, 1, 1, 1, 2],
                vec![2, 1, 0, 0, 0, 0, 0, 0, 1, 2],
                vec![2, 1, 0, 0, 0, 0, 0, 0, 1, 2],
                vec![2, 1, 1, 1, 0, 0, 1, 1, 1, 2],
                vec![4, 4, 4, 1, 0, 0, 1, 4, 4, 4],
                vec![4, 2, 4, 1, 1, 1, 1, 4, 2, 4],
                vec![4, 4, 4, 2, 2, 2, 2, 4, 4, 4],
            ],
            row_clues: vec![
                Some(8),
                Some(5),
                Some(4),
                Some(1),
                Some(3),
                Some(1),
                Some(4),
                Some(14),
                Some(13),
                Some(2),
            ],
            col_clues: vec![
                Some(4),
                Some(8),
                Some(17),
                Some(1),
                Some(1),
                Some(2),
                Some(7),
                Some(5),
                Some(8),
                Some(2),
            ],
        };
        let url = cspuz_rs_puzzles::puzzles::pills::serialize_problem(&problem).unwrap();
        let response = solve_problem_json_from_bytes(url.as_bytes());
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"));

        let board = &response["description"];
        assert_eq!(board["height"].as_usize(), Some(10));
        assert_eq!(board["width"].as_usize(), Some(10));
        assert_eq!(board["isUnique"].as_bool(), Some(true));

        let data = board["data"].members().collect::<Vec<_>>();
        // The value-10 pill occupies the top-left corner of row 8 (row index 7).
        assert!(data.iter().any(|entry| {
            entry["x"].as_usize() == Some(1)
                && entry["y"].as_usize() == Some(17)
                && entry["item"]["data"].as_str() == Some("10")
        }));
        // The value-4 pill occupies (6,9) in grid coordinates -> (13,19) on the board.
        assert!(data.iter().any(|entry| {
            entry["x"].as_usize() == Some(13)
                && entry["y"].as_usize() == Some(19)
                && entry["item"]["data"].as_str() == Some("4")
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
    fn solve_problem_dispatches_fourwinds() {
        let response = solve_problem_json_from_bytes(b"https://puzz.link/p?fourwinds/3/3/04c00c02");
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
        let data = response["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("green") && entry["item"].as_str() == Some("arrowRight")
        }));
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("green") && entry["item"].as_str() == Some("arrowUp")
        }));
    }

    #[test]
    fn solve_problem_dispatches_japanese_sums_with_zeroes() {
        // 1x2 with digits 0..1 and clues col0=[0], col1=[1], row0=[1].  The
        // unique solution is (0,0)=0, (0,1)=1; the 0 is a real digit in this
        // variant, not a shaded blank.  The plain Japanese Sums solver would
        // report "no answer".
        let response =
            solve_problem_json_from_bytes(b"https://puzz.link/p?japanesesumswithzeroes/2/1/1/011");
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
        let data = response["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("green")
                && entry["item"]["kind"].as_str() == Some("text")
                && entry["item"]["data"].as_str() == Some("0")
        }));
        assert!(data.iter().any(|entry| {
            entry["color"].as_str() == Some("green")
                && entry["item"]["kind"].as_str() == Some("text")
                && entry["item"]["data"].as_str() == Some("1")
        }));
        assert!(!data.iter().any(|entry| {
            entry["color"].as_str() == Some("green")
                && entry["item"]["kind"].as_str() == Some("block")
        }));
    }

    #[test]
    fn solve_problem_dispatches_japanese_arrows() {
        // A legal Japanese Arrows board has an arrow in every cell.  This is
        // a small all-given instance (all values are 1) that exercises the
        // normal cardinal directions without relying on the old arrowless
        // compatibility payload.
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?japanese_arrows/3/3/212121412131111111",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
    }

    #[test]
    fn japanese_arrows_backend_preserves_diagonal_arrow_kinds() {
        let response = solve_problem_json_from_bytes(
            b"https://puzz.link/p?japanese_arrows/2/2/-8001-7001-6001-5001",
        );
        let response = json::parse(&response).unwrap();
        assert_eq!(response["status"].as_str(), Some("ok"), "{}", response);
        let data = response["description"]["data"]
            .members()
            .collect::<Vec<_>>();
        assert!(data
            .iter()
            .any(|entry| { entry["item"].as_str() == Some("arrowUpLeft") }));
        assert!(data
            .iter()
            .any(|entry| { entry["item"].as_str() == Some("arrowDownRight") }));
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
