#![allow(static_mut_refs)]

use cspuz_solver_backend::{
    enumerate_answers_json_from_bytes, solve_custom_travelline_json_from_bytes,
    solve_problem_json_from_bytes,
};

static mut SHARED_ARRAY: Vec<u8> = vec![];
static mut INPUT_ARRAY: Vec<u8> = vec![];

fn write_shared_output(ret_string: String) -> *const u8 {
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
pub fn prepare_input_buffer(len: usize) -> *mut u8 {
    unsafe {
        INPUT_ARRAY.clear();
        INPUT_ARRAY.resize(len, 0);
        INPUT_ARRAY.as_mut_ptr()
    }
}

#[no_mangle]
pub fn solve_problem(url: *const u8, len: usize) -> *const u8 {
    let url = unsafe { std::slice::from_raw_parts(url, len) };
    write_shared_output(solve_problem_json_from_bytes(url))
}

#[no_mangle]
pub fn enumerate_answers_problem(url: *const u8, len: usize, num_max_answers: usize) -> *const u8 {
    let url = unsafe { std::slice::from_raw_parts(url, len) };
    write_shared_output(enumerate_answers_json_from_bytes(url, num_max_answers))
}

#[no_mangle]
pub fn solve_custom_travelline(payload: *const u8, len: usize) -> *const u8 {
    let payload = unsafe { std::slice::from_raw_parts(payload, len) };
    write_shared_output(solve_custom_travelline_json_from_bytes(payload))
}

fn main() {}
