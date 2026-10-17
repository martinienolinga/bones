//! Matériaux MTL : conversion vers nos matériaux et lecture des textures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::mesh::{Material, Texture};

/// Une même image utilisée par plusieurs matériaux n'est lue qu'une fois
/// (les octets sont partagés).
#[derive(Default)]
pub(super) struct TextureCache {
    loaded: HashMap<PathBuf, Option<Texture>>,
}

impl TextureCache {
    fn load(&mut self, base: Option<&Path>, reference: &str) -> Option<Texture> {
        let base = base?;
        let candidates = texture_candidates(base, reference);
        let path = candidates.into_iter().find(|p| p.is_file())?;
        self.loaded
            .entry(path.clone())
            .or_insert_with(|| Texture::from_file(&path).ok())
            .clone()
    }
}

/// Chemins possibles d'une texture citée dans un MTL : options (`-bm 1 …`)
/// ignorées, séparateurs Windows acceptés, et repli sur le seul nom.
fn texture_candidates(base: &Path, reference: &str) -> Vec<PathBuf> {
    let reference = reference.trim();
    let file = if reference.starts_with('-') {
        reference.split_whitespace().last().unwrap_or(reference)
    } else {
        reference
    };
    let file = file.replace('\\', "/");
    let mut out = vec![base.join(&file)];
    if let Some(name) = Path::new(&file).file_name() {
        out.push(base.join(name));
    }
    out
}

/// MTL → matériau PBR. La rugosité est déduite de l'exposant spéculaire
/// (`Ns`) par l'approximation usuelle Blinn-Phong → GGX.
pub(super) fn convert(m: &tobj::Material, base: Option<&Path>, cache: &mut TextureCache) -> Material {
    let [r, g, b] = m.diffuse.unwrap_or([0.8, 0.8, 0.8]);
    let alpha = m.dissolve.unwrap_or(1.0).clamp(0.0, 1.0);
    let roughness = m
        .shininess
        .map(|ns| (2.0 / (ns.max(0.0) + 2.0)).sqrt())
        .unwrap_or(0.9)
        .clamp(0.04, 1.0);
    Material {
        name: if m.name.is_empty() {
            "Material".into()
        } else {
            m.name.clone()
        },
        base_color: [r, g, b, alpha],
        base_color_texture: m.diffuse_texture.as_deref().and_then(|t| cache.load(base, t)),
        metallic: 0.0,
        roughness,
    }
}

/// Exposant spéculaire MTL correspondant à une rugosité.
pub(super) fn shininess(roughness: f32) -> f32 {
    let r = roughness.clamp(0.04, 1.0);
    (2.0 / (r * r) - 2.0).clamp(0.0, 1000.0)
}
