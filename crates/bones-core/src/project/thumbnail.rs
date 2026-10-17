//! Vignettes de la galerie : JPEG encodé une fois à l'enregistrement, stocké
//! sans recompression dans le projet.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageReader, Limits};

use super::ProjectError;

/// Qualité JPEG des vignettes.
const QUALITY: u8 = 85;
/// Côté maximal accepté au décodage (fichiers reçus d'ailleurs).
const MAX_SIDE: u32 = 8192;

/// Encode une image RGBA 8 bits (alpha ignoré : la vue 3D est opaque) en JPEG.
pub fn encode_thumbnail(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, ProjectError> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if width == 0 || height == 0 || expected != Some(rgba.len()) {
        return Err(ProjectError::Image(format!(
            "{} bytes do not match a {width}×{height} RGBA image",
            rgba.len()
        )));
    }
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let mut out = Vec::with_capacity(rgb.len() / 8);
    JpegEncoder::new_with_quality(&mut out, QUALITY)
        .encode(&rgb, width, height, ExtendedColorType::Rgb8)
        .map_err(|e| ProjectError::Image(e.to_string()))?;
    Ok(out)
}

/// Décode une vignette en RGBA 8 bits : (largeur, hauteur, pixels).
pub fn decode_thumbnail(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), ProjectError> {
    let error = |e: image::ImageError| ProjectError::Image(e.to_string());
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(ProjectError::Io)?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let rgba = reader.decode().map_err(error)?.into_rgba8();
    Ok((rgba.width(), rgba.height(), rgba.into_raw()))
}
