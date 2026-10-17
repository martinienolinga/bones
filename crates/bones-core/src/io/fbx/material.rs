//! Matériaux et textures FBX : images intégrées (`Video/Content`) ou
//! fichiers cherchés à côté du modèle.

use std::collections::HashMap;
use std::path::Path;

use super::document::{Document, Object, Props, child_attr};
use crate::mesh::{Material, Texture};

/// Propriétés de matériau pouvant porter la texture de couleur de base.
const COLOR_SLOTS: [&str; 4] = ["DiffuseColor", "Maya|baseColor", "base_color", "BaseColor"];

pub(super) struct Materials {
    pub list: Vec<Material>,
    /// Identifiant FBX → indice dans `list`.
    pub index: HashMap<i64, usize>,
}

pub(super) fn read(doc: &Document, base: Option<&Path>) -> Materials {
    let mut textures: HashMap<i64, Option<Texture>> = HashMap::new();
    let mut list = Vec::new();
    let mut index = HashMap::new();
    let mut ids: Vec<i64> = doc.of_class("Material").map(|o| o.id).collect();
    ids.sort_unstable();
    for id in ids {
        let o = &doc.objects[&id];
        let p = Props::of(&o.node);
        let color = p
            .vec3("DiffuseColor")
            .or_else(|| p.vec3("Diffuse"))
            .unwrap_or(glam::DVec3::splat(0.8));
        let factor = p.f64("DiffuseFactor").unwrap_or(1.0);
        let opacity = p
            .f64("Opacity")
            .or_else(|| p.f64("TransparencyFactor").map(|t| 1.0 - t))
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let shininess = p
            .f64("ShininessExponent")
            .or_else(|| p.f64("Shininess"))
            .unwrap_or(20.0);
        let texture = COLOR_SLOTS
            .iter()
            .flat_map(|slot| doc.children_with_property(id, slot))
            .find(|t| t.class == "Texture")
            .and_then(|t| {
                textures
                    .entry(t.id)
                    .or_insert_with(|| load_texture(doc, t, base))
                    .clone()
            });
        let c = color * factor;
        index.insert(id, list.len());
        list.push(Material {
            name: if o.name.is_empty() {
                "Matériau".into()
            } else {
                o.name.clone()
            },
            base_color: [c.x as f32, c.y as f32, c.z as f32, opacity as f32],
            base_color_texture: texture,
            metallic: 0.0,
            roughness: ((2.0 / (shininess.max(0.0) + 2.0)).sqrt() as f32).clamp(0.04, 1.0),
        });
    }
    Materials { list, index }
}

fn load_texture(doc: &Document, texture: &Object, base: Option<&Path>) -> Option<Texture> {
    let text = |node: &fbxcel::tree::v7400::NodeHandle, name: &str| {
        child_attr(node, name)
            .and_then(|a| a.get_string())
            .map(str::to_owned)
    };
    let mut names = vec![
        text(&texture.node, "RelativeFilename"),
        text(&texture.node, "FileName"),
    ];
    // Image intégrée dans une « Video » reliée à la texture.
    for video in doc.children_of(texture.id, "Video") {
        if let Some(bytes) = child_attr(&video.node, "Content").and_then(|a| a.get_binary())
            && !bytes.is_empty()
        {
            let name = text(&video.node, "RelativeFilename").unwrap_or_else(|| texture.name.clone());
            return Some(Texture::new(file_name(&name), bytes.to_vec()));
        }
        names.push(text(&video.node, "RelativeFilename"));
        names.push(text(&video.node, "Filename"));
    }
    let base = base?;
    names.into_iter().flatten().find_map(|n| {
        let n = n.replace('\\', "/");
        [base.join(&n), base.join(file_name(&n))]
            .into_iter()
            .find(|p| p.is_file())
            .and_then(|p| Texture::from_file(&p).ok())
    })
}

fn file_name(path: &str) -> String {
    let path = path.replace('\\', "/");
    path.rsplit('/').next().unwrap_or(&path).to_owned()
}
