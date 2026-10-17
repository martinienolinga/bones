//! Matériaux, textures et images glTF (export). Une même image partagée par
//! plusieurs matériaux n'est écrite qu'une fois.

use std::sync::Arc;

use serde_json::{Value, json};

use super::buffer::Builder;
use crate::Mesh;

/// Ajoute `materials`, `textures`, `images` et `samplers` au document.
/// Retourne les images à écrire à part (mode texte).
pub(super) fn write(
    b: &mut Builder,
    doc: &mut Value,
    mesh: &Mesh,
    external: Option<&str>,
) -> Vec<(String, Arc<[u8]>)> {
    if mesh.materials.is_empty() {
        return Vec::new();
    }
    let mut images: Vec<Value> = Vec::new();
    let mut seen: Vec<*const u8> = Vec::new();
    let mut files = Vec::new();
    let mut materials = Vec::with_capacity(mesh.materials.len());

    for m in &mesh.materials {
        let mut pbr = json!({
            "baseColorFactor": m.base_color,
            "metallicFactor": m.metallic,
            "roughnessFactor": m.roughness,
        });
        if let Some(tex) = &m.base_color_texture {
            let key = tex.bytes.as_ptr();
            let index = match seen.iter().position(|p| *p == key) {
                Some(i) => i,
                None => {
                    let image = match external {
                        Some(stem) => {
                            let file = format!("{stem}_tex{}.{}", images.len(), tex.extension());
                            files.push((file.clone(), tex.bytes.clone()));
                            json!({ "uri": file, "mimeType": tex.mime })
                        }
                        None => {
                            let view = b.view(&tex.bytes, None);
                            json!({ "bufferView": view, "mimeType": tex.mime })
                        }
                    };
                    images.push(image);
                    seen.push(key);
                    images.len() - 1
                }
            };
            pbr["baseColorTexture"] = json!({ "index": index });
        }
        let mut material = json!({ "name": m.name, "pbrMetallicRoughness": pbr, "doubleSided": true });
        if m.base_color[3] < 1.0 {
            material["alphaMode"] = json!("BLEND");
        }
        materials.push(material);
    }

    doc["materials"] = json!(materials);
    if !images.is_empty() {
        // Une texture par image, toutes avec l'échantillonneur par défaut
        // (linéaire, mipmaps, répétition).
        let textures: Vec<Value> = (0..images.len())
            .map(|i| json!({ "source": i, "sampler": 0 }))
            .collect();
        doc["textures"] = json!(textures);
        doc["images"] = json!(images);
        doc["samplers"] = json!([{ "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497 }]);
    }
    files
}
