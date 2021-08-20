use std::ops::Range;
use std::fmt::{Debug, Formatter};

type TetroShape4 = [ [ bool; 4 ]; 4];
type TetroShape3 = [ [ bool; 3 ]; 3];

pub enum TetroShape {
    Even(TetroShape4),
    Odd(TetroShape3)
}

pub type Tetromino = usize;

const TI: Tetromino = 1;
pub const TL: Tetromino = 7;

pub const RANGE: Range<Tetromino> = TI..TL;


// #[repr(usize)]
// #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
// pub enum Tetromino {
//     None = 0, I, O, T, S, Z, J, L
// }
//
// impl Tetromino {
//     pub fn range() -> Range<usize> {
//         Tetromino::I as usize..Tetromino::L as usize
//     }
// }
//
// impl Into<usize> for Tetromino {
//     fn into(self) -> usize {
//         self as usize
//     }
// }
//
// impl Tetromino {
//     pub fn shape(self) -> TetroShape {
//         ALL[<Tetromino as Into<usize>>::into(self)]
//     }
//
//
//     pub fn color(self) -> Color {
//         TETRO_COLORS[<Tetromino as Into<usize>>::into(self)]
//     }
// }

pub const NAMES: [&str; 8] = [
    "NONE",
    "I",
    "O",
    "T",
    "S",
    "Z",
    "J",
    "L",
];

pub const ALL: [TetroShape; 8] = [
    TetroShape::Odd(TETRO_NONE),
    TetroShape::Even(TETRO_I),
    TetroShape::Even(TETRO_O),
    TetroShape::Odd(TETRO_T),
    TetroShape::Odd(TETRO_S),
    TetroShape::Odd(TETRO_Z),
    TetroShape::Odd(TETRO_J),
    TetroShape::Odd(TETRO_L),
];

type Color = [ f32; 4 ];

 fn col_f32( c: u8 ) -> f32 {
    255.0 / (c as f32)
}

fn rgba( rgba: u32 ) -> Color {
    [
        (((rgba >> 24) as u8) as f32) / 255.0,
        (((rgba >> 16) as u8) as f32) / 255.0,
        (((rgba >>  8) as u8) as f32) / 255.0,
        (((rgba >>  0) as u8) as f32) / 255.0,
    ]
}

 fn rgb( rgb: u32 ) -> Color {
    rgba(rgb << 8 | 0xff)
}

lazy_static! {
pub static ref Colors: Vec<Color> = vec!(
    [  0.0, 0.0, 0.0, 0.0 ],
    rgba(0x00C0C0ff),
    rgba(0xFDE01Aff),
    rgba(0x732982ff),
    rgba(0x007940ff),
    rgb(0xD12229),
    rgba(0x24408Eff),
    rgba(0xf68a1eff),

);
}

pub const TETRO_COLORS: [Color; 8] = [
    [  0.0, 0.0, 0.0, 0.0 ],
    //rgb(0xD12229),
    [ 0xD1 as f32 / 255.0, 0x22 as f32 / 255.0, 0x29 as f32 / 255.0, 1.0 ],
    // rgba(0xf68a1eff),
    [ 0xF6 as f32 / 255.0, 0x8A as f32 / 255.0, 0x1E as f32 / 255.0, 1.0 ],
    [ 0xFD as f32 / 255.0, 0xE0 as f32 / 255.0, 0x1A as f32 / 255.0, 1.0 ],
    [ 0x00 as f32 / 255.0, 0x79 as f32 / 255.0, 0x40 as f32 / 255.0, 1.0 ],
    [ 0x00 as f32 / 255.0, 0xC0 as f32 / 255.0, 0xC0 as f32 / 255.0, 1.0 ],
    [ 0x24 as f32 / 255.0, 0x40 as f32 / 255.0, 0x8E as f32 / 255.0, 1.0 ],
    [ 0x73 as f32 / 255.0, 0x29 as f32 / 255.0, 0x82 as f32 / 255.0, 1.0 ],
];

/*
        [ 255.0 / 0xD1 as f32, 255.0 / 0x22 as f32, 255.0 / 0x29 as f32 ],
        [ 255.0 / 0xF6 as f32, 255.0 / 0x8A as f32, 255.0 / 0x1E as f32 ],
        [ 255.0 / 0xFD as f32, 255.0 / 0xE0 as f32, 255.0 / 0x1A as f32 ],
        [ 255.0 / 0x00 as f32, 255.0 / 0x79 as f32, 255.0 / 0x40 as f32 ],
        [ 255.0 / 0x00 as f32, 255.0 / 0xC0 as f32, 255.0 / 0xC0 as f32 ],
        [ 255.0 / 0x24 as f32, 255.0 / 0x40 as f32, 255.0 / 0x8E as f32 ],
        [ 255.0 / 0x73 as f32, 255.0 / 0x29 as f32, 255.0 / 0x82 as f32 ],
*/

const XX: bool = true;
const __: bool = false;

pub const TETRO_NONE: TetroShape3 = [
    [__,__,__],
    [__,__,__],
    [__,__,__],
];

pub const TETRO_I: TetroShape4 = [
    [__,__,__,__],
    [XX,XX,XX,XX],
    [__,__,__,__],
    [__,__,__,__],
];

pub const TETRO_O: TetroShape4 = [
    [__,__,__,__],
    [__,XX,XX,__],
    [__,XX,XX,__],
    [__,__,__,__],
];

pub const TETRO_T: TetroShape3 = [
    [__,__,__],
    [__,XX,__],
    [XX,XX,XX],
];

pub const TETRO_S: TetroShape3 = [
    [__,__,__],
    [__,XX,XX],
    [XX,XX,__],
];

pub const TETRO_Z: TetroShape3 = [
    [__,__,__],
    [XX,XX,__],
    [__,XX,XX],
];

pub const TETRO_J: TetroShape3 = [
    [__,__,XX],
    [__,__,XX],
    [__,XX,XX],
];

pub const TETRO_L: TetroShape3 = [
    [XX,__,__],
    [XX,__,__],
    [XX,XX,__],
];