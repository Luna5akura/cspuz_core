use crate::util;
use cspuz_rs::graph;
use cspuz_rs::solver::Solver;

#[derive(Clone,Copy,Debug,PartialEq,Eq)] pub enum Mark { None, Sheep, Wolf }
pub fn solve_wolves_and_sheep(clues:&[Vec<Option<i32>>], marks:&[Vec<Mark>])->Option<graph::BoolGridEdgesIrrefutableFacts>{
 let (h,w)=util::infer_shape(clues); let mut s=Solver::new(); let e=&graph::BoolGridEdges::new(&mut s,(h,w)); s.add_answer_key_bool(&e.horizontal); s.add_answer_key_bool(&e.vertical); graph::single_cycle_grid_edges(&mut s,e);
 for y in 0..h {for x in 0..w {if let Some(n)=clues[y][x] {s.add_expr(e.cell_neighbors((y,x)).count_true().eq(n));}}}
 let _=marks; s.irrefutable_facts().map(|f|f.get(e))
}
