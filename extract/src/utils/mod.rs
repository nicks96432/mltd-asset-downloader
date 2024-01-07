mod read_ext;

use image::DynamicImage;
use rabex::objects::classes::Rectf;

pub use read_ext::*;

pub fn solve_puzzle(name: &str, img: &DynamicImage, pieces: &Vec<Rectf>) -> Vec<DynamicImage> {
    let pieces = pieces
        .into_iter()
        .map(|x| img.crop_imm(x.x as u32, x.y as u32, x.width as u32, x.height as u32))
        .collect::<Vec<_>>();

    todo!("solve puzzle")
}
