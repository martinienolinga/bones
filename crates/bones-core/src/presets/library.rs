//! Bibliothèque intégrée : uniquement des préréglages procéduraux créés
//! pour l'application (licence CC0), qui s'adaptent à tout squelette et ne
//! pèsent presque rien. Aucune donnée de capture tierce n'est embarquée.
//!
//! Le catalogue (noms, catégories, générateurs, paramètres) est le fichier
//! `assets/presets/builtin.json`, inclus à la compilation.

use std::sync::LazyLock;

use serde::Deserialize;

use super::Generator;
use super::model::{Category, Preset, PresetInfo, PresetKind, PresetSource, Procedural};
use super::params::Params;
use super::reference::RefRig;
use super::roles::detect;

/// Auteur et licence des préréglages intégrés.
pub const AUTHOR: &str = "PaintSharp Bones";
pub const LICENSE: &str = "CC0-1.0";

const CATALOG: &str = include_str!("../../assets/presets/builtin.json");

/// Ligne du catalogue.
#[derive(Deserialize)]
struct Row {
    id: String,
    name: String,
    category: Category,
    group: String,
    tags: Vec<String>,
    description: String,
    generator: Generator,
    #[serde(default)]
    params: Params,
    #[serde(default)]
    reference: Option<RefRig>,
}

/// Préréglages intégrés (calculés une fois).
pub fn builtin() -> &'static [Preset] {
    static LIBRARY: LazyLock<Vec<Preset>> = LazyLock::new(build);
    &LIBRARY
}

/// Préréglage intégré par identifiant.
pub fn find_builtin(id: &str) -> Option<&'static Preset> {
    builtin().iter().find(|p| p.info.id == id)
}

fn build() -> Vec<Preset> {
    let rows: Vec<Row> = serde_json::from_str(CATALOG).expect("catalogue des préréglages intégrés");
    rows.into_iter()
        .map(|row| {
            let procedural = Procedural {
                generator: row.generator,
                params: row.params,
                reference: row.reference,
            };
            let (frames, root_motion) = measure(&procedural);
            let prefix = match row.category {
                Category::Human => "human",
                Category::Animals => "animals",
                Category::Vegetation => "vegetation",
                Category::Misc => "misc",
            };
            Preset {
                info: PresetInfo {
                    id: format!("builtin.{prefix}.{}", row.id),
                    name: row.name,
                    category: row.category,
                    group: row.group,
                    tags: row.tags,
                    description: row.description,
                    author: AUTHOR.into(),
                    license: LICENSE.into(),
                    source: PresetSource::Builtin,
                    rig_type: row.generator.rig_type(),
                    fps: 30.0,
                    frames,
                    looping: row.generator.looping(),
                    root_motion,
                    procedural: true,
                    ..PresetInfo::default()
                },
                kind: PresetKind::Procedural(procedural),
            }
        })
        .collect()
}

/// Durée (images à 30 i/s) et déplacement du corps, mesurés sur le
/// squelette de référence.
pub(crate) fn measure(p: &Procedural) -> (u32, bool) {
    let skeleton = p.reference().skeleton();
    let roles = detect(&skeleton);
    match super::procedural::measure(p.generator, &p.params, &skeleton, &roles) {
        Some((duration, travel)) => (((duration * 30.0).round() as u32).max(1), travel > 0.01),
        None => (0, false),
    }
}
