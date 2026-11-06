//! Unpacks `Texture2D` and `Sprite`.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageFormat};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::error::{Repr, Result};
use crate::unity::{Bundle, Class, Object, Rect};
use crate::unpack::puzzle::{ex4c_name, solve_puzzle, stack_strips};

/// The textures that are the strips of an image, by the name of the image. The strips of an
/// image are in the order from top to bottom.
pub(super) type StripGroups<'a> = BTreeMap<&'a str, Vec<&'a Object>>;

/// Finds the textures of a bundle that are the strips of an four-panel comics.
///
/// An image is found only if its strips are numbered from 0 without gaps, and it has at least
/// 2 of them.
pub(super) fn strip_groups(bundle: &impl Bundle) -> StripGroups<'_> {
    let mut positions: BTreeMap<&str, BTreeMap<u32, &Object>> = BTreeMap::new();
    for texture in bundle.objects().iter().filter(|o| o.class == Class::Texture2D) {
        if let Some((name, position)) = ex4c_name(&texture.name) {
            positions.entry(name).or_default().insert(position, texture);
        }
    }

    positions
        .into_iter()
        .filter(|(_, strips)| strips.len() >= 2 && strips.keys().zip(0..).all(|(&p, i)| p == i))
        .map(|(name, strips)| (name, strips.into_values().collect()))
        .collect()
}

/// Writes the images of a [`Class::Texture2D`] to `output_dir`, which are named after the
/// texture and numbered.
///
/// If the texture is a strip of an four-panel comic in `strips`, the whole image is written
/// named after the image.
pub(super) fn extract(
    bundle: &impl Bundle,
    texture: &Object,
    output_dir: &Path,
    format: ImageFormat,
    strips: &StripGroups<'_>,
) -> Result<()> {
    let strip_group = ex4c_name(&texture.name).and_then(|(name, _)| strips.get_key_value(name));
    match strip_group {
        // writes the whole image once for the first texture only
        Some((name, group)) if group[0] == texture => {
            return extract_strips(bundle, name, group, output_dir, format);
        }
        Some((name, group)) if group.contains(&texture) => {
            tracing::trace!(
                texture = texture.name,
                image = name,
                "strip is written with the first one"
            );
            return Ok(());
        }
        _ => (),
    }

    // Texture2D requires all relavent Sprites to be extracted
    let sprites = bundle.objects().iter().filter(|o| o.class == Class::Sprite);
    let images = solve(bundle, texture, sprites)?;

    // rayon workers do not inherit the span of the caller
    let span = tracing::Span::current();
    images.into_iter().enumerate().par_bridge().try_for_each(|(i, image)| -> Result<()> {
        let _entered = span.enter();
        let path =
            output_dir.join(format!("{}_{}.{}", texture.name, i, format.extensions_str()[0]));

        write_image(&image, &path, format)
    })
}

fn write_image(image: &DynamicImage, path: &Path, format: ImageFormat) -> Result<()> {
    tracing::info!(output_path = %path.display(), "writing image");

    let io_error = |e| Repr::io("write image", Some(path), e);
    let mut file = BufWriter::new(File::create(path).map_err(io_error)?);
    image.write_to(&mut file, format).map_err(Repr::Image)?;
    file.flush().map_err(io_error)?;

    Ok(())
}

/// Writes the image that `strips` are cut from to `output_dir`, named after `name`.
fn extract_strips(
    bundle: &impl Bundle,
    name: &str,
    strips: &[&Object],
    output_dir: &Path,
    format: ImageFormat,
) -> Result<()> {
    tracing::debug!(image = name, strips = strips.len(), "stacking strips of a four-panel comic");
    let images = strips.iter().map(|strip| bundle.texture(strip)).collect::<Result<Vec<_>>>()?;

    if images.iter().any(|image| image.width() != images[0].width()) {
        tracing::warn!(name, "strips are not equally wide, padding the narrower ones");
    }

    let path = output_dir.join(format!("{name}.{}", format.extensions_str()[0]));

    write_image(&stack_strips(&images), &path, format)
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

    let mut rects = Vec::new();
    for object in sprites {
        let sprite = bundle.sprite(object)?;
        if sprite.texture_id != texture.path_id {
            continue;
        }
        tracing::trace!(sprite = object.name, rect = ?sprite.rect, "found sprite of the texture");

        let Rect { x, y, width, height } = sprite.rect;
        let exceeds =
            |start: u32, len: u32, bound: u32| start.checked_add(len).is_none_or(|e| e > bound);
        if exceeds(x, width, image.width()) || exceeds(y, height, image.height()) {
            tracing::warn!(name = object.name, "sprite rect is out of the texture, skipping");
            continue;
        }

        // pieces are in the order of the number at the end of the name, and the ones without
        // a number come last
        let piece_number = object
            .name
            .rsplit_once('_')
            .and_then(|(_, n)| n.parse::<u32>().ok())
            .unwrap_or(u32::MAX);
        rects.push((piece_number, &object.name, image.view(x, y, width, height)));
    }
    rects.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    let pieces = rects.into_iter().map(|(_, _, piece)| piece).collect::<Vec<_>>();
    tracing::debug!(
        texture = texture.name,
        width = image.width(),
        height = image.height(),
        pieces = pieces.len(),
        "solving puzzle"
    );
    let images = match solve_puzzle(&texture.name, &image, &pieces) {
        Ok(images) => images,
        Err(e) => {
            tracing::warn!(error = ?e, filename = texture.name, "using original image");
            vec![upright]
        }
    };

    Ok(images)
}
