use std::ops::Range;

type TetroShape4 = [ [ bool; 4 ]; 4];
type TetroShape3 = [ [ bool; 3 ]; 3];

#[derive(Clone)]
pub enum TetroShape {
    Even(TetroShape4),
    Odd(TetroShape3)
}

impl TetroShape {
    pub fn is_solid(&self, x: usize, y: usize) -> bool {
        match self {
            TetroShape::Odd(t) if x < 3 && y < 3 => t[y][x],
            TetroShape::Even(t) if x < 4 && y < 4 => t[y][x],
            _ => false
        }
    }

    pub fn rotated(&self, steps: u8) -> TetroShape {
        match self {
            TetroShape::Odd(t) => TetroShape::Odd(rotated3(t.clone(), steps)),
            TetroShape::Even(t) => TetroShape::Even(rotated4(t.clone(), steps)),
        }
    }
}

fn rotated3(m: TetroShape3, steps: u8) -> TetroShape3 {
    match steps {
        0 => m,
        _ => {
            let [[a, b, c],
                 [d, e, f],
                 [g, h, i]] = m;
            rotated3([[ g, d, a ],
                          [ h, e, b ],
                          [ i, f, c ]],steps - 1)
        }
    }
}

fn rotated4(x: TetroShape4, steps: u8) -> TetroShape4 {
    match steps {
        0 => x,
        _ => {
            let [[a, b, c, d],
                 [e, f, g, h],
                 [i, j, k, l],
                 [m, n, o, p]] = x;
            rotated4([[ m, i, e, a ],
                         [ n, j, f, b ],
                         [ o, k, g, c ],
                         [ p, l, h, d ]],steps - 1)
        }
    }
}

pub type Tetromino = usize;

const TI: Tetromino = 1;
pub const TL: Tetromino = 7;

pub const RANGE: Range<Tetromino> = TI..TL;

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