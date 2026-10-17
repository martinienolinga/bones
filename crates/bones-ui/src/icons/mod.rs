//! Icônes en couleur, dessinées en SVG (dégradés, reflets) et rastérisées
//! une seule fois par `resvg` dans un atlas de textures egui.
//!
//! Chaque famille d'outils a sa teinte (voir [`hue`]) : os en ambre, IK en
//! bleu, rotation en violet, épingles en vert, animation en or, physique en
//! sarcelle, fichiers en bleu ciel, affichage en indigo.

mod atlas;
pub mod hue;
mod svg;

use egui::{Color32, Painter, Rect};

pub use atlas::rasterize;

/// Icônes de l'application. L'ordre de déclaration est l'indice dans
/// l'atlas (voir [`all`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Gallery,
    Actions,
    Adjust,
    Bone,
    BoneAdd,
    Move,
    Rotate,
    Pin,
    Select,
    Delete,
    Undo,
    Redo,
    Play,
    Pause,
    PrevKey,
    NextKey,
    Key,
    KeyRemove,
    Mirror,
    Symmetrize,
    AutoRig,
    Weights,
    Eye,
    Hierarchy,
    Reset,
    Ballistic,
    Ghost,
    Trajectory,
    CenterOfMass,
    Export,
    Import,
    Save,
    Folder,
    Settings,
    Ik,
    Fk,
    Close,
    Check,
    Frame,
    Grid,
    Mesh,
    Marker,
    Chain,
    Pose,
    Edit,
    Lock,
    Home,
    Ortho,
}

const ALL: [Icon; 48] = [
    Icon::Gallery,
    Icon::Actions,
    Icon::Adjust,
    Icon::Bone,
    Icon::BoneAdd,
    Icon::Move,
    Icon::Rotate,
    Icon::Pin,
    Icon::Select,
    Icon::Delete,
    Icon::Undo,
    Icon::Redo,
    Icon::Play,
    Icon::Pause,
    Icon::PrevKey,
    Icon::NextKey,
    Icon::Key,
    Icon::KeyRemove,
    Icon::Mirror,
    Icon::Symmetrize,
    Icon::AutoRig,
    Icon::Weights,
    Icon::Eye,
    Icon::Hierarchy,
    Icon::Reset,
    Icon::Ballistic,
    Icon::Ghost,
    Icon::Trajectory,
    Icon::CenterOfMass,
    Icon::Export,
    Icon::Import,
    Icon::Save,
    Icon::Folder,
    Icon::Settings,
    Icon::Ik,
    Icon::Fk,
    Icon::Close,
    Icon::Check,
    Icon::Frame,
    Icon::Grid,
    Icon::Mesh,
    Icon::Marker,
    Icon::Chain,
    Icon::Pose,
    Icon::Edit,
    Icon::Lock,
    Icon::Home,
    Icon::Ortho,
];

/// Toutes les icônes (aperçus, tests).
pub fn all() -> &'static [Icon] {
    &ALL
}

impl Icon {
    /// Document SVG complet de l'icône.
    pub fn svg(self) -> String {
        let (body, hue) = svg::source(self);
        hue::document(hue, body)
    }

    /// Couleur principale de la famille (lueurs, états actifs).
    pub fn tone(self) -> Color32 {
        let (_, hue) = svg::source(self);
        parse_hex(hue.dark)
    }
}

/// Dessine `icon` dans le carré inscrit dans `rect`. `tint` multiplie les
/// couleurs : blanc pour l'icône d'origine, blanc translucide pour atténuer.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, tint: Color32) {
    atlas::paint(painter, rect, icon, tint);
}

/// Prépare l'atlas d'icônes pour la densité de pixels courante (sinon il
/// est créé au premier dessin).
pub fn install(ctx: &egui::Context) {
    atlas::prepare(ctx);
}

fn parse_hex(hex: &str) -> Color32 {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0xffffff);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}
