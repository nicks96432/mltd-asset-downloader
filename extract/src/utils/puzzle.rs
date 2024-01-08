use std::collections::HashMap;
use std::error::Error;

use image::{imageops, DynamicImage};
use lazy_regex::regex::Regex;
use lazy_regex::{regex, Lazy};
use rabex::objects::classes::Rectf;

pub fn solve_puzzle(
    texture_name: &str,
    img: &DynamicImage,
    pieces: &HashMap<String, Rectf>,
) -> Result<Vec<DynamicImage>, Box<dyn Error>> {
    let (puzzle_name, _) = NAME_PUZZLE_MAP
        .iter()
        .find(|(_, r)| r.is_match(texture_name))
        .ok_or("cannot find puzzle")?;

    let puzzle = PIECE_MAP.get(puzzle_name).ok_or("puzzle not implemented")?;

    let mut puzzle_imgs = Vec::new();

    for i in 0..puzzle.img_count {
        let piece_count = pieces.len() / puzzle.img_count;
        let mut puzzle_img = DynamicImage::new_rgba8(puzzle.width, puzzle.height);

        for j in 0..piece_count {
            let piece = pieces
                .get(&format!("{}_{}", texture_name, i * piece_count + j))
                .ok_or("cannot find piece")?;
            let mut piece = img.crop_imm(
                piece.x as u32,
                piece.y as u32,
                piece.width as u32,
                piece.height as u32,
            );

            if puzzle.pieces[j].flip {
                piece = piece.flipv();
            }

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
static NAME_PUZZLE_MAP: Lazy<Vec<(&'static str, &'static Lazy<Regex>)>> = Lazy::new(|| {
    vec![
        ("bg",                regex!(r"^bg2d_[a-z0-9][0-9]{2,3}$")),
        ("bg_thumb",          regex!(r"^bg2d_[a-z0-9][0-9]{2,3}_thumb$")),
        ("card",              regex!(r"^[0-9]{3}[a-z]{3}[0-9]{4}_[0-9]$")),
        ("card_big",          regex!(r"^[0-9]{3}[a-z]{3}[0-9]{4}_[0-9]_bg$")),
        ("event_bg",          regex!(r"^mycard_bg_(?:event|other)_[0-9]{4}$")),
        ("event_logo",        regex!(r"^mycard_logo_event_[0-9]{4}$")),
        // ("four_comic",       regex!(r"^ex4c[0-9]{5}$")),
        ("four_comic_thumb",  regex!(r"^ex4c[0-9]{5}_thumb$")),
        ("one_comic",         regex!(r"^hitokoma_[0-9]{2,6}$")),
        ("one_comic_thumb",   regex!(r"^hitokoma_[0-9]{2,6}_thumb$")),
        ("icon",              regex!(r"^icon_[0-9]{3}[a-z]{3}[0-9]{4}$")),
        ("jacket",            regex!(r"^jacket_[a-z0-9]{6}$")),
        ("jacket_small",      regex!(r"^jacket_[a-z0-9]{6}_small$")),
        ("live_skill",        regex!(r"^live_skill_[0-9]{3}$")),
        ("loading_character", regex!(r"^loadingchara(?:_[0-9]{2}){1,2}?$")),
        ("whiteboard",        regex!(r"^exwb[0-9]{7}$")),
    ]
});

#[derive(Debug, Clone)]
struct Piece {
    x: i64,
    y: i64,
    flip: bool,
    rotate: u32,
}

#[derive(Debug, Clone)]
struct Puzzle {
    width: u32,
    height: u32,
    pieces: Vec<Piece>,
    img_count: usize,
}

static PIECE_MAP: Lazy<HashMap<&'static str, Puzzle>> = Lazy::new(|| {
    HashMap::from([
        (
            "card",
            Puzzle {
                width: 640,
                height: 800,
                pieces: vec![
                    Piece { x: 0, y: 0, flip: true, rotate: 0 },
                    Piece { x: 0, y: 510, flip: true, rotate: 0 },
                    Piece { x: 380, y: 510, flip: true, rotate: 0 },
                    Piece { x: 380, y: 728, flip: true, rotate: 0 },
                    Piece { x: 498, y: 728, flip: true, rotate: 0 },
                    Piece { x: 616, y: 728, flip: true, rotate: 0 },
                ],
                img_count: 2,
            },
        ),
        (
            "card_big",
            Puzzle {
                width: 1280,
                height: 720,
                pieces: vec![
                    Piece { x: 0, y: 0, flip: true, rotate: 0 },
                    Piece { x: 1022, y: 0, flip: true, rotate: 270 },
                ],
                img_count: 1,
            },
        ),
    ])
});
