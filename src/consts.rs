#![allow(dead_code)]

use std::fmt;

#[macro_export]
macro_rules! SQ {
    ($row: expr, $col: expr) => {
        /* '8' is used here because chess game always is an 8x8 game */
        ($row * 8) + $col
    };
}

#[macro_export]
macro_rules! ROW {
    ($sq: expr) => {
        ($sq >> 3)
    };
}

#[macro_export]
macro_rules! COL {
    ($sq: expr) => {
        ($sq & 7)
    };
}

#[macro_export]
macro_rules! FLIP_SQ {
    ($sq: expr) => {
        ($sq ^ 56)
    };
}

pub use {COL, FLIP_SQ, ROW, SQ};

#[rustfmt::skip]
const PIECE_CHAR: [char; 13] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k', ' '];
#[rustfmt::skip]
const STR_COORDS: [&str; 65] = [
    "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8",
    "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
    "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6",
    "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5",
    "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4",
    "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3",
    "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2",
    "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1", " "
];

#[derive(Copy, Clone, PartialEq)]
pub enum PieceColor {
    Light,
    Dark,
    Both,
}

#[derive(Copy, Clone, PartialEq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Piece {
    LP,
    LN,
    LB,
    LR,
    LQ,
    LK,
    DP,
    DN,
    DB,
    DR,
    DQ,
    DK,
}

impl Piece {
    pub fn from_tuple(color: usize, piece_type: usize) -> Option<Self> {
        if color != (PieceColor::Both as usize) {
            return None;
        }
        Self::from_num(color * 6 + piece_type)
    }

    pub fn from_num(piece_num: usize) -> Option<Self> {
        assert!(piece_num <= 12);
        if piece_num == 12 {
            return None;
        }
        let piece = match piece_num {
            0 => Self::LP,
            1 => Self::LN,
            2 => Self::LB,
            3 => Self::LR,
            4 => Self::LQ,
            5 => Self::LK,
            6 => Self::DP,
            7 => Self::DN,
            8 => Self::DB,
            9 => Self::DR,
            10 => Self::DQ,
            11 => Self::DK,
            _ => unreachable!("Piece::from_num() should only contain these values: [0-11]"),
        };
        Some(piece)
    }

    pub fn from_char(piece_char: char) -> Option<Self> {
        assert!(PIECE_CHAR.into_iter().any(|x| x == piece_char));
        let char_index = PIECE_CHAR
            .into_iter()
            .position(|x| x == piece_char)
            .unwrap();
        Self::from_num(char_index)
    }

    pub fn to_tuple(piece: Option<Piece>) -> (usize, usize) {
        if piece.is_none() {
            return (2, 0);
        }
        let piece_num = piece.unwrap() as usize;
        (piece_num / 6, piece_num % 6)
    }

    pub fn to_num(piece: Option<Piece>) -> usize {
        if piece.is_none() {
            return 12;
        }
        piece.unwrap() as usize
    }

    pub fn to_char(piece: Option<Piece>) -> char {
        PIECE_CHAR[Self::to_num(piece)]
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone)]
pub enum Direction {
    North = 8,
    South = -8,
    West = -1,
    East = 1,
    NE = 9,     // NORTH + EAST
    NW = 7,     // NORTH + WEST
    SE = -7,    // SOUTH + EAST
    SW = -9,    // SOUTH + WEST
    NE_N = 17,  // 2(NORTH) + EAST -> 'KNIGHT ONLY'
    NE_E = 10,  // NORTH + 2(EAST) -> 'KNIGHT ONLY'
    NW_N = 15,  // 2(NORTH) + WEST -> 'KNIGHT ONLY'
    NW_W = 6,   // NORTH + 2(WEST) -> 'KNIGHT ONLY'
    SE_S = -15, // 2(SOUTH) + EAST -> 'KNIGHT ONLY'
    SE_E = -6,  // SOUTH + 2(EAST) -> 'KNIGHT ONLY'
    SW_S = -17, // 2(SOUTH) + WEST -> 'KNIGHT ONLY'
    SW_W = -10, // SOUTH + 2(WEST) -> 'KNIGHT ONLY'
}

#[rustfmt::skip]
#[derive(Copy, Clone, PartialEq)]
pub enum Sq {
    A8, B8, C8, D8, E8, F8, G8, H8,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A1, B1, C1, D1, E1, F1, G1, H1,
    NoSq,
}

impl fmt::Display for Sq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Sq::to_string(*self))
    }
}

impl Sq {
    pub fn from_str(sq_str: &str) -> Sq {
        assert!(sq_str.len() == 2);
        let file = (sq_str.chars().nth(0).unwrap() as u8) - b'a';
        let rank = 8 - (sq_str.chars().nth(1).unwrap() as u8 - b'0');
        Self::from_num(SQ!(rank, file) as usize)
    }

    pub fn from_num(sq_num: usize) -> Self {
        match sq_num {
            0 => Self::A8,
            1 => Self::B8,
            2 => Self::C8,
            3 => Self::D8,
            4 => Self::E8,
            5 => Self::F8,
            6 => Self::G8,
            7 => Self::H8,
            8 => Self::A7,
            9 => Self::B7,
            10 => Self::C7,
            11 => Self::D7,
            12 => Self::E7,
            13 => Self::F7,
            14 => Self::G7,
            15 => Self::H7,
            16 => Self::A6,
            17 => Self::B6,
            18 => Self::C6,
            19 => Self::D6,
            20 => Self::E6,
            21 => Self::F6,
            22 => Self::G6,
            23 => Self::H6,
            24 => Self::A5,
            25 => Self::B5,
            26 => Self::C5,
            27 => Self::D5,
            28 => Self::E5,
            29 => Self::F5,
            30 => Self::G5,
            31 => Self::H5,
            32 => Self::A4,
            33 => Self::B4,
            34 => Self::C4,
            35 => Self::D4,
            36 => Self::E4,
            37 => Self::F4,
            38 => Self::G4,
            39 => Self::H4,
            40 => Self::A3,
            41 => Self::B3,
            42 => Self::C3,
            43 => Self::D3,
            44 => Self::E3,
            45 => Self::F3,
            46 => Self::G3,
            47 => Self::H3,
            48 => Self::A2,
            49 => Self::B2,
            50 => Self::C2,
            51 => Self::D2,
            52 => Self::E2,
            53 => Self::F2,
            54 => Self::G2,
            55 => Self::H2,
            56 => Self::A1,
            57 => Self::B1,
            58 => Self::C1,
            59 => Self::D1,
            60 => Self::E1,
            61 => Self::F1,
            62 => Self::G1,
            63 => Self::H1,
            _ => unreachable!("Sq::from_num() should only contain these values: [0-63]"),
        }
    }

    pub fn to_string(sq_num: Self) -> String {
        STR_COORDS[sq_num as usize].to_string()
    }
}
