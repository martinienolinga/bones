//! Tests des formats : allers-retours et fichiers témoins minimaux.

mod bvh;
mod corpus;
mod fbx;
mod gltf;
mod obj;
mod ply;
mod stl;

use std::path::PathBuf;

/// Dossier temporaire propre à un test.
pub(super) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-io-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Petite image PNG 2×2 valide.
pub(super) fn tiny_png() -> Vec<u8> {
    let mut out = Vec::new();
    let img = image::RgbaImage::from_raw(
        2,
        2,
        vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255],
    )
    .unwrap();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}
