use crate::board::{Board, BoardKind, Item, ItemKind};
use crate::uniqueness::{is_unique, UniquenessCheckable};
use cspuz_rs_puzzles::puzzles::hex_masyu;

impl UniquenessCheckable for hex_masyu::HexMasyuAnswer {
    fn is_unique(&self) -> bool {
        self.right.is_unique() && self.bottom_left.is_unique() && self.bottom_right.is_unique()
    }
}

pub fn solve(url: &str) -> Result<Board, &'static str> {
    use hex_masyu::{HexDir, HexMasyuClue};

    let problem = hex_masyu::deserialize_problem(url).ok_or("invalid url")?;
    let ans = hex_masyu::solve_hex_masyu(&problem).ok_or("no answer")?;

    let height = problem.len();
    let width = problem[0].len();
    let mut board = Board::new(BoardKind::Empty, height, width, is_unique(&ans));

    for y in 0..height {
        for x in 0..width {
            match problem[y][x] {
                Some(HexMasyuClue::None) | None => (),
                Some(HexMasyuClue::White) => {
                    board.push(Item::cell(y, x, "black", ItemKind::Circle))
                }
                Some(HexMasyuClue::Black) => {
                    board.push(Item::cell(y, x, "black", ItemKind::FilledCircle))
                }
            }
        }
    }

    // The answer overlay is expressed per cell: for each used edge going out of
    // a board cell, a lineTo item pointing at the adjacent cell. The UI maps
    // (cell, direction) back to the shared hex edge.
    let get_edge = |y: i32, x: i32, dir: HexDir| -> Option<bool> {
        if !hex_masyu::is_in_board(height, width, y, x) {
            return Some(false);
        }
        let (y, x) = (y as usize, x as usize);
        match dir {
            HexDir::Right => {
                if x + 1 < width && hex_masyu::is_in_board(height, width, y as i32, (x + 1) as i32)
                {
                    ans.right[y][x]
                } else {
                    Some(false)
                }
            }
            HexDir::Left => {
                if x >= 1 && hex_masyu::is_in_board(height, width, y as i32, (x - 1) as i32) {
                    ans.right[y][x - 1]
                } else {
                    Some(false)
                }
            }
            HexDir::BottomLeft => {
                if y + 1 < height
                    && hex_masyu::is_in_board(height, width, (y + 1) as i32, x as i32)
                {
                    ans.bottom_left[y][x]
                } else {
                    Some(false)
                }
            }
            HexDir::BottomRight => {
                if y + 1 < height
                    && x + 1 < width
                    && hex_masyu::is_in_board(height, width, (y + 1) as i32, (x + 1) as i32)
                {
                    ans.bottom_right[y][x]
                } else {
                    Some(false)
                }
            }
            HexDir::TopRight => {
                if y >= 1 && hex_masyu::is_in_board(height, width, (y - 1) as i32, x as i32) {
                    ans.bottom_left[y - 1][x]
                } else {
                    Some(false)
                }
            }
            HexDir::TopLeft => {
                if y >= 1
                    && x >= 1
                    && hex_masyu::is_in_board(height, width, (y - 1) as i32, (x - 1) as i32)
                {
                    ans.bottom_right[y - 1][x - 1]
                } else {
                    Some(false)
                }
            }
        }
    };

    for y in 0..height {
        for x in 0..width {
            if !hex_masyu::is_in_board(height, width, y as i32, x as i32) {
                continue;
            }
            for &dir in &hex_masyu::ALL_HEX_DIRS {
                if get_edge(y as i32, x as i32, dir) == Some(true) {
                    let (dy, dx) = dir.offset();
                    board.push(Item::cell(y, x, "green", ItemKind::LineTo(dy, dx)));
                }
            }
        }
    }

    Ok(board)
}
