use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::is_unique;
use cspuz_rs_puzzles::puzzles::wolves_and_sheep::{self, Mark};

pub fn solve(url: &str) -> Result<Board, &'static str> {
    let (clues, marks) = wolves_and_sheep::deserialize_problem(url).ok_or("invalid url")?;
    let answer = wolves_and_sheep::solve_wolves_and_sheep(&clues, &marks).ok_or("no answer")?;
    let h = clues.len();
    let w = clues.first().map(|r| r.len()).unwrap_or(0);
    if h == 0 || clues.iter().any(|r| r.len() != w) { return Err("invalid answer"); }
    let mut board = Board::new(BoardKind::DotGrid, h, w, is_unique(&answer));
    for y in 0..h { for x in 0..w {
        if let Some(n) = clues[y][x] { board.push(Item::cell(y, x, "black", ItemKind::Num(n))); }
        match marks[y][x] {
            Mark::Sheep => board.push(Item::cell(y, x, "black", ItemKind::Text("S"))),
            Mark::Wolf => board.push(Item::cell(y, x, "black", ItemKind::Text("W"))),
            Mark::None => {}
        }
    }}
    for y in 0..h { for x in 0..=w { if let Some(b) = answer.vertical[y][x] { board.push(Item { y:y*2+1, x:x*2, color:"green", kind: if b { ItemKind::Wall } else { ItemKind::Cross } }); } }}
    for y in 0..=h { for x in 0..w { if let Some(b) = answer.horizontal[y][x] { board.push(Item { y:y*2, x:x*2+1, color:"green", kind: if b { ItemKind::Wall } else { ItemKind::Cross } }); } }}
    Ok(board)
}
