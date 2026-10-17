//! Modèle de données d'un préréglage : fiche descriptive (lisible seule,
//! pour la galerie) et contenu (clés sur un squelette source, ou générateur
//! procédural et ses paramètres).

use serde::{Deserialize, Serialize};

use super::Generator;
use super::expression::ExpressionData;
use super::params::Params;
use super::reference::RefRig;
use super::roles::{RigType, RoleMap};
use crate::{BoneId, Clip, Skeleton};

/// Catégorie de contenu (rayon de la galerie).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Human,
    Animals,
    Vegetation,
    /// Créatures et objets : queues, tentacules, cordes, pendules, machines.
    Misc,
    /// Expressions faciales (pose unique du visage) pour les êtres vivants.
    FacialExpression,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Human,
        Category::Animals,
        Category::Vegetation,
        Category::Misc,
        Category::FacialExpression,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Human => "Human",
            Category::Animals => "Animals",
            Category::Vegetation => "Vegetation",
            Category::Misc => "Creatures & Misc",
            Category::FacialExpression => "Facial Expressions",
        }
    }

    /// Catégorie par défaut d'un type de rig (préréglages importés).
    pub fn for_rig(rig: RigType) -> Category {
        match rig {
            RigType::Humanoid => Category::Human,
            RigType::Quadruped | RigType::Bird | RigType::Fish | RigType::Serpent | RigType::MultiLegged => {
                Category::Animals
            }
            RigType::Plant => Category::Vegetation,
            RigType::Chain | RigType::Any => Category::Misc,
        }
    }
}

/// Provenance d'un préréglage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PresetSource {
    /// Livré avec l'application.
    Builtin,
    /// Créé par l'utilisateur.
    User,
    /// Importé d'un autre logiciel ; `format` : « BVH », « glTF », « FBX »…
    Imported { format: String },
}

/// Fiche d'un préréglage : tout ce que la galerie affiche, sans le contenu.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetInfo {
    /// Identifiant stable (`builtin.human.walk`, `user.…`).
    pub id: String,
    pub name: String,
    pub category: Category,
    /// Sous-catégorie libre : « Locomotion », « Quadrupeds », « Trees »…
    pub group: String,
    pub tags: Vec<String>,
    pub description: String,
    pub author: String,
    /// Licence (identifiant SPDX quand il existe : `CC0-1.0`, `CC-BY-4.0`…).
    pub license: String,
    pub source: PresetSource,
    /// Famille de squelette visée.
    pub rig_type: RigType,
    pub fps: f32,
    /// Durée en images (d'un cycle pour une boucle).
    pub frames: u32,
    pub looping: bool,
    /// Le mouvement déplace le personnage (marche, saut en longueur…).
    pub root_motion: bool,
    pub procedural: bool,
    /// Os du squelette source (0 pour un préréglage procédural).
    pub bones: u32,
    /// Secondes Unix (UTC).
    pub created: u64,
    pub modified: u64,
}

impl Default for PresetInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Untitled".into(),
            category: Category::Misc,
            group: String::new(),
            tags: Vec::new(),
            description: String::new(),
            author: String::new(),
            license: String::new(),
            source: PresetSource::User,
            rig_type: RigType::Any,
            fps: 30.0,
            frames: 0,
            looping: false,
            root_motion: false,
            procedural: false,
            bones: 0,
            created: 0,
            modified: 0,
        }
    }
}

impl PresetInfo {
    /// Durée en secondes.
    pub fn duration(&self) -> f32 {
        self.frames as f32 / self.fps.max(1.0)
    }

    /// Le texte apparaît dans le nom, le groupe, les étiquettes ou la description.
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        q.is_empty()
            || [&self.name, &self.group, &self.description]
                .iter()
                .any(|s| s.to_lowercase().contains(&q))
            || self.tags.iter().any(|t| t.to_lowercase().contains(&q))
    }
}

/// Animation par clés, conçue sur un squelette source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyframeData {
    pub skeleton: Skeleton,
    /// Rôles des os source (détectés, éventuellement corrigés).
    pub roles: RoleMap,
    pub clip: Clip,
    /// Os animés ; vide : tous. Les autres gardent leur pose à l'application.
    #[serde(default)]
    pub bones: Vec<BoneId>,
}

/// Générateur et paramètres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Procedural {
    pub generator: Generator,
    #[serde(default)]
    pub params: Params,
    /// Squelette d'aperçu (par défaut celui du générateur).
    #[serde(default)]
    pub reference: Option<RefRig>,
}

impl Procedural {
    pub fn reference(&self) -> RefRig {
        self.reference.unwrap_or(self.generator.reference())
    }
}

/// Contenu d'un préréglage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PresetKind {
    Keyframes(Box<KeyframeData>),
    Procedural(Procedural),
    /// Expression faciale : pose unique du visage, appliquée via
    /// [`crate::Editor::apply_expression`] (voir [`super::expression`]).
    Expression(Box<ExpressionData>),
}

/// Préréglage complet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub info: PresetInfo,
    pub kind: PresetKind,
}

/// Section de la galerie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Category(Category),
    /// Préréglages créés par l'utilisateur.
    MyPresets,
    /// Préréglages importés d'autres logiciels.
    Imported,
}

impl Section {
    pub const ALL: [Section; 7] = [
        Section::Category(Category::Human),
        Section::Category(Category::Animals),
        Section::Category(Category::Vegetation),
        Section::Category(Category::Misc),
        Section::Category(Category::FacialExpression),
        Section::MyPresets,
        Section::Imported,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::Category(c) => c.label(),
            Section::MyPresets => "My Presets",
            Section::Imported => "Imported",
        }
    }

    /// Le préréglage apparaît dans cette section (un préréglage utilisateur
    /// apparaît aussi dans sa catégorie).
    pub fn contains(self, info: &PresetInfo) -> bool {
        match self {
            Section::Category(c) => {
                info.category == c && !matches!(info.source, PresetSource::Imported { .. })
            }
            Section::MyPresets => info.source == PresetSource::User,
            Section::Imported => matches!(info.source, PresetSource::Imported { .. }),
        }
    }
}

/// Identifiant unique pour un nouveau préréglage (heure et compteur).
pub fn new_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}.{nanos:x}{n:04x}")
}
