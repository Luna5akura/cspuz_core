use crate::board::{Board, BoardKind};
use crate::uniqueness::Uniqueness;
pub fn solve(url: &str) -> Result<Board, &'static str> {
    let p = url.split('?').nth(1).ok_or("invalid url")?;
    let v: Vec<_> = p.split('/').collect();
    if v.len() < 3 { return Err("invalid url"); }
    let w = v[1].parse().map_err(|_| "invalid url")?;
    let h = v[2].parse().map_err(|_| "invalid url")?;
    Ok(Board::new(BoardKind::Grid, h, w, Uniqueness::NotApplicable))
}
