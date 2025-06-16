//! Puzzle handling.

use image::{DynamicImage, GenericImageView, SubImage, imageops};
use thiserror::Error as ThisError;

use crate::error::{Error, Repr, Result};

#[derive(Debug, ThisError)]
pub(crate) enum PuzzleError {
    #[error("puzzle not found for texture {0}")]
    PuzzleNotFound(String),

    #[error("puzzle {0} is not implemented yet")]
    NotImplemented(String),
}

impl From<PuzzleError> for Error {
    fn from(value: PuzzleError) -> Self {
        Repr::from(value).into()
    }
}

/// Solves a puzzle.
///
/// # Errors
///
/// Returns [`crate::Error`] with [`crate::ErrorKind::Puzzle`] if the puzzle cannot be solved.
pub fn solve_puzzle(
    texture_name: &str,
    img: &DynamicImage,
    pieces: &[SubImage<&DynamicImage>],
) -> Result<Vec<DynamicImage>> {
    let (puzzle_name, _) = NAME_PUZZLE_MAP
        .iter()
        .find(|(_, r)| regex::Regex::new(r).map_or_else(|_| false, |r| r.is_match(texture_name)))
        .ok_or_else(|| PuzzleError::PuzzleNotFound(texture_name.to_string()))?;

    let puzzle = piece_map(puzzle_name)
        .ok_or_else(|| PuzzleError::NotImplemented((*puzzle_name).to_string()))?;

    let mut puzzle_imgs = Vec::new();

    for i in 0..puzzle.img_count {
        let piece_count = pieces.len() / puzzle.img_count;
        let mut puzzle_img = DynamicImage::new_rgba8(puzzle.width, puzzle.height);

        for j in 0..piece_count {
            let piece_idx = i * piece_count + j;
            let piece = pieces.get(piece_idx).ok_or(Repr::OutOfRange(piece_idx, pieces.len()))?;

            let mut piece = img
                .crop_imm(piece.offsets().0, piece.offsets().1, piece.width(), piece.height())
                .flipv();

            match puzzle.pieces[j].rotate {
                90 => piece = piece.rotate90(),
                180 => piece = piece.rotate180(),
                270 => piece = piece.rotate270(),
                _ => (),
            }

            imageops::overlay(&mut puzzle_img, &piece, puzzle.pieces[j].x, puzzle.pieces[j].y);
        }

        puzzle_imgs.push(puzzle_img);
    }

    Ok(puzzle_imgs)
}

#[rustfmt::skip]
const NAME_PUZZLE_MAP: &[(&str, &str)] =  &[
        ("bg"                , r"^bg2d_[a-z0-9][0-9]{2,3}$"),
        ("bg_thumb"          , r"^bg2d_[a-z0-9][0-9]{2,3}_thumb$"),
        ("card"              , r"^[0-9]{3}[a-z]{3}[0-9]{4}_[0-9]$"),
        ("card_big"          , r"^[0-9]{3}[a-z]{3}[0-9]{4}_[0-9]_bg$"),
        ("event_bg"          , r"^mycard_bg_(?:event|other)_[0-9]{4}$"),
        ("event_logo"        , r"^mycard_logo_event_[0-9]{4}$"),
        // ("four_comic",      r"^ex4c[0-9]{5}$"),
        ("four_comic_thumb"  , r"^ex4c[0-9]{5}_thumb$"),
        ("one_comic"         , r"^hitokoma_[0-9]{2,6}$"),
        ("one_comic_thumb"   , r"^hitokoma_[0-9]{2,6}_thumb$"),
        ("icon"              , r"^icon_[0-9]{3}[a-z]{3}[0-9]{4}$"),
        ("jacket"            , r"^jacket_[a-z0-9]{6}$"),
        ("jacket_small"      , r"^jacket_[a-z0-9]{6}_small$"),
        ("live_skill"        , r"^live_skill_[0-9]{3}$"),
        ("loading_character" , r"^loadingchara(?:_[0-9]{2}){1,2}?$"),
        ("whiteboard"        , r"^exwb[0-9]{7}$"),
];

#[derive(Debug, Clone)]
struct Piece {
    x: i64,
    y: i64,
    rotate: u32,
}

#[derive(Debug, Clone)]
struct Puzzle {
    width: u32,
    height: u32,
    pieces: Vec<Piece>,
    img_count: usize,
}

fn piece_map(puzzle_name: &str) -> Option<Puzzle> {
    match puzzle_name {
        "card" => Some(Puzzle {
            width: 640,
            height: 800,
            pieces: vec![
                Piece { x: 0, y: 0, rotate: 0 },
                Piece { x: 0, y: 510, rotate: 0 },
                Piece { x: 380, y: 510, rotate: 0 },
                Piece { x: 380, y: 728, rotate: 0 },
                Piece { x: 498, y: 728, rotate: 0 },
                Piece { x: 616, y: 728, rotate: 0 },
            ],
            img_count: 2,
        }),
        "card_big" => Some(Puzzle {
            width: 1280,
            height: 720,
            pieces: vec![Piece { x: 0, y: 0, rotate: 0 }, Piece { x: 1022, y: 0, rotate: 270 }],
            img_count: 1,
        }),
        _ => None,
    }
}
