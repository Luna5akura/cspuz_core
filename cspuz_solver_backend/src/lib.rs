#![allow(static_mut_refs)] // TODO: remove this

extern crate cspuz_rs;

pub mod board;
mod custom_travelline;
mod puzzle;
mod uniqueness;

use board::Board;
use cspuz_rs::serializer::{get_kudamono_url_info_detailed, url_to_puzzle_kind};
pub use puzzle::{list_penpa_edit_puzzles, list_puzzles_for_enumerate, list_puzzles_for_solve};

static mut SHARED_ARRAY: Vec<u8> = vec![];
static mut INPUT_ARRAY: Vec<u8> = vec![];

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

#[no_mangle]
fn prepare_input_buffer(len: usize) -> *mut u8 {
    unsafe {
        INPUT_ARRAY.clear();
        INPUT_ARRAY.resize(len, 0);
        INPUT_ARRAY.as_mut_ptr()
    }
}

#[no_mangle]
fn solve_problem(url: *const u8, len: usize) -> *const u8 {
    let url = unsafe { std::slice::from_raw_parts(url, len) };
    let ret_string = solve_problem_json_from_bytes(url);

    let ret_len = ret_string.len();
    unsafe {
        SHARED_ARRAY.clear();
        SHARED_ARRAY.reserve(4 + ret_len);
        SHARED_ARRAY.push((ret_len & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 8) & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 16) & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 24) & 0xff) as u8);
        SHARED_ARRAY.extend_from_slice(ret_string.as_bytes());
        SHARED_ARRAY.as_ptr()
    }
}

#[no_mangle]
fn enumerate_answers_problem(url: *const u8, len: usize, num_max_answers: usize) -> *const u8 {
    let url = unsafe { std::slice::from_raw_parts(url, len) };
    let ret_string = enumerate_answers_json_from_bytes(url, num_max_answers);

    let ret_len = ret_string.len();
    unsafe {
        SHARED_ARRAY.clear();
        SHARED_ARRAY.reserve(4 + ret_len);
        SHARED_ARRAY.push((ret_len & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 8) & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 16) & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 24) & 0xff) as u8);
        SHARED_ARRAY.extend_from_slice(ret_string.as_bytes());
        SHARED_ARRAY.as_ptr()
    }
}

#[no_mangle]
fn solve_custom_travelline(payload: *const u8, len: usize) -> *const u8 {
    let payload = unsafe { std::slice::from_raw_parts(payload, len) };
    let ret_string = solve_custom_travelline_json_from_bytes(payload);

    let ret_len = ret_string.len();
    unsafe {
        SHARED_ARRAY.clear();
        SHARED_ARRAY.reserve(4 + ret_len);
        SHARED_ARRAY.push((ret_len & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 8) & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 16) & 0xff) as u8);
        SHARED_ARRAY.push(((ret_len >> 24) & 0xff) as u8);
        SHARED_ARRAY.extend_from_slice(ret_string.as_bytes());
        SHARED_ARRAY.as_ptr()
    }
}
