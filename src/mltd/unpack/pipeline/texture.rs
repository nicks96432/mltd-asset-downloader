//! Unpacks `Texture2D` and `Sprite`.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageFormat};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::error::{Repr, Result};
use crate::unity::{Bundle, Class, Object, Rect};
use crate::unpack::puzzle::solve_puzzle;

/// Writes the images of a [`Class::Texture2D`] to `output_dir`, which are named after the
/// texture and numbered.
pub(super) fn extract(
    bundle: &impl Bundle,
    texture: &Object,
    output_dir: &Path,
    format: ImageFormat,
) -> Result<()> {
    // Texture2D requires all relavent Sprites to be extracted
    let sprites = bundle.objects().iter().filter(|o| o.class == Class::Sprite);
    let images = solve(bundle, texture, sprites)?;

    images.into_iter().enumerate().par_bridge().try_for_each(|(i, image)| -> Result<()> {
        let path =
            output_dir.join(format!("{}_{}.{}", texture.name, i, format.extensions_str()[0]));
        tracing::info!(output_path = %path.display(), "writing image");

        let io_error = |e| Repr::io("write image", Some(&path), e);
        let mut file = BufWriter::new(File::create(&path).map_err(io_error)?);
        image.write_to(&mut file, format).map_err(Repr::Image)?;
        file.flush().map_err(io_error)?;

        Ok(())
    })
}

/// Gets the images of a texture, with the puzzle solved if it is one.
fn solve<'a>(
    bundle: &impl Bundle,
    texture: &Object,
    sprites: impl Iterator<Item = &'a Object>,
) -> Result<Vec<DynamicImage>> {
    let upright = bundle.texture(texture)?;

    // the rectangles of sprites are measured from the bottom left corner
    let image = upright.flipv();

    let mut rects = BTreeMap::new();
    for object in sprites {
        let sprite = bundle.sprite(object)?;
        if sprite.texture_id != texture.path_id {
            continue;
        }

        let Rect { x, y, width, height } = sprite.rect;
        if u64::from(x) + u64::from(width) > u64::from(image.width())
            || u64::from(y) + u64::from(height) > u64::from(image.height())
        {
            tracing::warn!(name = object.name, "sprite rect is out of the texture, skipping");
            continue;
        }

        let Some(sprite_id) = object.name.rsplit_once('_').and_then(|(_, n)| n.parse::<u32>().ok())
        else {
            tracing::warn!(name = object.name, "sprite name has no piece number, skipping");
            continue;
        };
        rects.insert(sprite_id, image.view(x, y, width, height));
    }

    let images = match solve_puzzle(&texture.name, &image, &rects.into_values().collect::<Vec<_>>())
    {
        Ok(images) => images,
        Err(e) => {
            tracing::warn!(error = ?e, filename = texture.name, "using original image");
            vec![upright]
        }
    };

    Ok(images)
}
