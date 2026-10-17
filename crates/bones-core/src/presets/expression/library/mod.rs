//! Bibliothèque intégrée d'expressions faciales (données CC0, créées pour
//! l'application). Chaque expression est une pose unique du visage, par os du
//! visage ; elle s'adapte à tout squelette qui a ces os (les absents sont
//! ignorés). Rangées par groupe pour la galerie : émotions (Ekman + mépris),
//! expressions courantes, visèmes (Preston Blair), animaux.

mod animals;
mod common;
mod emotions;
mod visemes;

use super::data::ExpressionData;
use crate::presets::library::{AUTHOR, LICENSE};
use crate::presets::model::{Category, Preset, PresetInfo, PresetKind, PresetSource};
use crate::presets::roles::RigType;

/// Fiche d'une expression intégrée avant construction du [`Preset`].
pub(super) struct Spec {
    id: &'static str,
    name: &'static str,
    group: &'static str,
    tags: &'static [&'static str],
    description: &'static str,
    rig: RigType,
    data: ExpressionData,
}

impl Spec {
    pub(super) fn new(
        id: &'static str,
        name: &'static str,
        group: &'static str,
        tags: &'static [&'static str],
        description: &'static str,
        rig: RigType,
        data: ExpressionData,
    ) -> Self {
        Self {
            id,
            name,
            group,
            tags,
            description,
            rig,
            data,
        }
    }
}

/// Toutes les expressions intégrées, prêtes pour la galerie.
pub(crate) fn builtin() -> Vec<Preset> {
    let mut specs = Vec::new();
    specs.extend(emotions::all());
    specs.extend(common::all());
    specs.extend(visemes::all());
    specs.extend(animals::all());
    specs.into_iter().map(to_preset).collect()
}

fn to_preset(s: Spec) -> Preset {
    let bones = s.data.bones.len() as u32;
    Preset {
        info: PresetInfo {
            id: format!("builtin.face.{}", s.id),
            name: s.name.into(),
            category: Category::FacialExpression,
            group: s.group.into(),
            tags: s.tags.iter().map(|t| (*t).to_string()).collect(),
            description: s.description.into(),
            author: AUTHOR.into(),
            license: LICENSE.into(),
            source: PresetSource::Builtin,
            rig_type: s.rig,
            fps: 30.0,
            frames: 1,
            looping: false,
            root_motion: false,
            procedural: false,
            bones,
            ..PresetInfo::default()
        },
        kind: PresetKind::Expression(Box::new(s.data)),
    }
}
