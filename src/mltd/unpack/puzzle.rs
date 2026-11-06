//! Texture puzzles handling.
//!
//! An image that does not fit in a texture is cut into pieces, which are the sprites of the
//! texture. However, nothing in the bundle tells how to put them back together, so we have
//! to put them back manually.

use std::sync::LazyLock;

use image::{DynamicImage, GenericImageView, SubImage, imageops};
use thiserror::Error as ThisError;

use crate::error::{Error as CrateError, Repr, Result};

#[derive(Debug, ThisError)]
pub(crate) enum Error {
    #[error("puzzle not found for texture {0}")]
    PuzzleNotFound(String),
}

impl From<Error> for CrateError {
    fn from(value: Error) -> Self {
        Repr::from(value).into()
    }
}

/// Solves a puzzle.
///
/// Pieces are given in the order of their sprite names, which is the order of the puzzle pieces
/// recognized by the name of the texture.
///
/// # Errors
///
/// Returns [`crate::Error`] with [`crate::ErrorKind::Puzzle`] if the puzzle cannot be solved.
pub fn solve_puzzle(
    texture_name: &str,
    img: &DynamicImage,
    pieces: &[SubImage<&DynamicImage>],
) -> Result<Vec<DynamicImage>> {
    static CARD_REGEX: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(CARD_PATTERN).expect("valid pattern"));

    if let Some(puzzle_img) = solve_simple_puzzle(img, pieces) {
        return Ok(vec![puzzle_img]);
    }

    if !CARD_REGEX.is_match(texture_name) {
        return Err(Error::PuzzleNotFound(texture_name.to_string()).into());
    }

    let puzzle = CARD_PUZZLE;
    let mut puzzle_imgs = Vec::new();

    for i in 0..puzzle.img_count {
        let piece_count = pieces.len() / puzzle.img_count;
        let mut puzzle_img = DynamicImage::new_rgba8(puzzle.width, puzzle.height);

        for j in 0..piece_count {
            let piece_idx = i * piece_count + j;
            let piece = pieces.get(piece_idx).ok_or(Repr::OutOfRange(piece_idx, pieces.len()))?;

            let mut piece = crop_piece(img, piece);

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

/// Crops a piece out of the image of a texture, whose origin is the bottom left corner, and
/// flips it vertically.
fn crop_piece(img: &DynamicImage, piece: &SubImage<&DynamicImage>) -> DynamicImage {
    let (x, y) = piece.offsets();

    img.crop_imm(x, y, piece.width(), piece.height()).flipv()
}

/// Solves the puzzle of an 16:9 image, which is cut into two pieces.
///
/// The main piece takes the width of the texture except for the padding of a pixel at each
/// side. The strip that is the rest of the image is stored rotated, so its width is the height
/// of the main piece, and it is rotated back and put to the right of the main piece.
///
/// # Returns
///
/// [`None`] if the pieces are not cut this way.
fn solve_simple_puzzle(
    img: &DynamicImage,
    pieces: &[SubImage<&DynamicImage>],
) -> Option<DynamicImage> {
    let [first, second] = pieces else {
        return None;
    };

    let area =
        |piece: &SubImage<&DynamicImage>| u64::from(piece.width()) * u64::from(piece.height());
    let (main, strip) = if area(first) >= area(second) { (first, second) } else { (second, first) };

    if main.width() + 2 != img.width() || strip.width() != main.height() {
        return None;
    }

    let mut puzzle_img = DynamicImage::new_rgba8(main.width() + strip.height(), main.height());
    imageops::overlay(&mut puzzle_img, &crop_piece(img, main), 0, 0);
    imageops::overlay(
        &mut puzzle_img,
        &crop_piece(img, strip).rotate270(),
        i64::from(main.width()),
        0,
    );

    Some(puzzle_img)
}

/// Gets the name of the image and the position of a texture that is one of its strips, e.g.
/// `("ex4c_00001", 3)` for `ex4c_00001_3`.
#[must_use]
pub fn ex4c_name(texture_name: &str) -> Option<(&str, u32)> {
    static EX4C_REGEX: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(EX4C_PATTERN).expect("valid pattern"));

    let captures = EX4C_REGEX.captures(texture_name)?;

    Some((captures.get(1)?.as_str(), captures[2].parse().ok()?))
}

/// Stacks the strips of an image from top to bottom.
///
/// If the strips are not equally wide, they are aligned to the left, and the rest of the
/// narrower ones is transparent.
#[must_use]
pub fn stack_strips(strips: &[DynamicImage]) -> DynamicImage {
    let width = strips.iter().map(DynamicImage::width).max().unwrap_or(0);
    let height = strips.iter().map(DynamicImage::height).sum();

    let mut stacked = DynamicImage::new_rgba8(width, height);
    let mut y = 0;
    for strip in strips {
        imageops::overlay(&mut stacked, strip, 0, y);
        y += i64::from(strip.height());
    }

    stacked
}

/// Names of the textures of four-panel comics, each of which is a horizontal strip.
const EX4C_PATTERN: &str = r"^(ex4c_[0-9]{5})_([0-9]+)$";

/// Names of the textures of cards.
const CARD_PATTERN: &str = r"^[0-9]{3}[a-z]{3}[0-9]{4}_[0-9]$";

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
    pieces: &'static [Piece],
    img_count: usize,
}

/// A texture of a card has the pieces of 2 images.
const CARD_PUZZLE: Puzzle = Puzzle {
    width: 640,
    height: 800,
    pieces: &[
        Piece { x: 0, y: 0, rotate: 0 },
        Piece { x: 0, y: 510, rotate: 0 },
        Piece { x: 380, y: 510, rotate: 0 },
        Piece { x: 380, y: 728, rotate: 0 },
        Piece { x: 498, y: 728, rotate: 0 },
        Piece { x: 616, y: 728, rotate: 0 },
    ],
    img_count: 2,
};

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;
    use crate::ErrorKind;

    /// Makes an image whose neighbouring pixels differ, so that misplaced pieces are noticed.
    fn pattern(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(width, height, |x, y| {
            let byte = |v: u32| u8::try_from(v % 256).unwrap();

            Rgba([byte(x % 251), byte(y % 241), byte(x / 251 + y / 241), 255])
        }))
    }

    /// Cuts an image into a main piece and a strip and puts them in a texture of side `side`.
    ///
    /// Returns the image of the texture, whose origin is the bottom left corner, and the
    /// rectangles of the pieces `(x, y, width, height)`.
    fn cut(img: &DynamicImage, side: u32) -> (DynamicImage, [(u32, u32, u32, u32); 2]) {
        let height = img.height();
        let main_width = side - 2;
        let strip_width = img.width() - main_width;

        let main = img.crop_imm(0, 0, main_width, height);
        let strip = img.crop_imm(main_width, 0, strip_width, height).rotate90();

        let mut texture = DynamicImage::new_rgba8(side, side);
        imageops::overlay(&mut texture, &main.flipv(), 1, 1);
        imageops::overlay(&mut texture, &strip.flipv(), 1, i64::from(height) + 2);

        (texture, [(1, 1, main_width, height), (1, height + 2, height, strip_width)])
    }

    fn views<'a>(
        texture: &'a DynamicImage,
        rects: &[(u32, u32, u32, u32)],
    ) -> Vec<SubImage<&'a DynamicImage>> {
        rects.iter().map(|&(x, y, w, h)| texture.view(x, y, w, h)).collect()
    }

    #[test]
    fn test_solve_strip_puzzle() {
        // (image width, image height, texture side)
        for (width, height, side) in
            [(1280, 720, 1024), (640, 370, 512), (650, 366, 512), (576, 324, 512)]
        {
            let full = pattern(width, height);
            let (texture, rects) = cut(&full, side);

            let solved = solve_puzzle("any_name", &texture, &views(&texture, &rects)).unwrap();

            assert!(solved.len() == 1 && solved[0] == full, "{width}x{height} is not restored");
        }
    }

    #[test]
    fn test_solve_simple_puzzle_any_order() {
        let full = pattern(1280, 720);
        let (texture, rects) = cut(&full, 1024);

        let reversed = [rects[1], rects[0]];
        let solved = solve_puzzle("any_name", &texture, &views(&texture, &reversed)).unwrap();

        assert!(solved.len() == 1 && solved[0] == full);
    }

    #[test]
    fn test_solve_not_simple_puzzle() {
        // the main piece does not take the width of the texture
        let (texture, rects) = cut(&pattern(1280, 720), 1024);
        let narrow = [(1, 1, 1000, 720), rects[1]];
        let error = solve_puzzle("bg2d_a0001", &texture, &views(&texture, &narrow)).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Puzzle);

        // two icons of the same size are not pieces of an image
        let texture = pattern(256, 512);
        let icons = [(1, 1, 104, 104), (107, 1, 104, 104)];
        let error = solve_puzzle("014mir0011_0_m", &texture, &views(&texture, &icons)).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Puzzle);

        // a single piece or more than two pieces are not solved either
        let single = [(1, 1, 254, 366)];
        assert!(solve_puzzle("jacket_harm4u", &texture, &views(&texture, &single)).is_err());
        let many = [(1, 1, 50, 50), (60, 1, 50, 50), (120, 1, 50, 50)];
        assert!(solve_puzzle("icon_014mir0123", &texture, &views(&texture, &many)).is_err());
    }

    /// Opens an image in the `tests` directory.
    fn open(name: &str) -> DynamicImage {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(name);

        image::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// Solves the puzzle of a real texture, whose pieces are the sprites in its bundle.
    ///
    /// `rects` are the rectangles of the sprites in the order of their names.
    fn assert_solves(
        texture_name: &str,
        texture: &str,
        rects: &[(u32, u32, u32, u32)],
        expected: &[&str],
    ) {
        // the rectangles of sprites are measured from the bottom left corner
        let texture = open(texture).flipv();

        let solved = solve_puzzle(texture_name, &texture, &views(&texture, rects)).unwrap();

        assert_eq!(solved.len(), expected.len(), "{texture_name}");
        for (solved, expected) in solved.iter().zip(expected) {
            assert!(solved.to_rgba8() == open(expected).to_rgba8(), "{expected} is not restored");
        }
    }

    #[test]
    fn test_solve_bg2d() {
        // sprites `bg2d_a1028_0` and `bg2d_a1028_1` in `tests/test_bg2d.unity3d`
        assert_solves(
            "bg2d_t0084",
            "test_bg2d.png",
            &[(1, 303, 1022, 720), (1, 1, 720, 258)],
            &["test_bg2d_processed.png"],
        );
    }

    #[test]
    fn test_solve_card() {
        // sprites `014mir0011_0_0` to `014mir0011_0_11` in `tests/test_image.unity3d`
        assert_solves(
            "014mir0011_0",
            "test_image.png",
            &[
                (1, 513, 640, 510),
                (643, 733, 380, 290),
                (643, 513, 260, 218),
                (905, 659, 118, 72),
                (905, 585, 118, 72),
                (905, 511, 24, 72),
                (1, 1, 640, 510),
                (643, 1, 380, 290),
                (643, 293, 260, 218),
                (905, 293, 118, 72),
                (905, 367, 118, 72),
                (999, 441, 24, 72),
            ],
            &["test_image_processed_0.png", "test_image_processed_1.png"],
        );
    }

    #[test]
    fn test_ex4c_name() {
        assert_eq!(ex4c_name("ex4c_00001_0"), Some(("ex4c_00001", 0)));
        assert_eq!(ex4c_name("ex4c_00044_12"), Some(("ex4c_00044", 12)));

        for name in ["ex4c00001", "ex4c_00001", "ex4c_00001_", "ex4c_00001_thumb", "xex4c_00001_1"]
        {
            assert_eq!(ex4c_name(name), None, "{name}");
        }
    }

    #[test]
    fn test_stack_strips() {
        let strips = [pattern(4, 2), pattern(3, 1), pattern(4, 3)];

        let stacked = stack_strips(&strips).to_rgba8();

        assert_eq!(stacked.dimensions(), (4, 6));
        assert_eq!(stacked.get_pixel(3, 0), strips[0].to_rgba8().get_pixel(3, 0));
        assert_eq!(stacked.get_pixel(2, 2), strips[1].to_rgba8().get_pixel(2, 0));
        assert_eq!(stacked.get_pixel(3, 2).0[3], 0, "the narrow strip is padded");
        assert_eq!(stacked.get_pixel(0, 5), strips[2].to_rgba8().get_pixel(0, 2));

        assert_eq!(stack_strips(&[]).dimensions(), (0, 0));
    }
}
