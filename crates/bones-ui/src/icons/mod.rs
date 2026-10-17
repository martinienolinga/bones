//! Icônes monochromes à la Procreate (glyphes SVG d'une seule couleur),
//! rastérisées une seule fois en blanc par `resvg` dans un atlas de textures
//! egui, puis teintées au dessin : gris clair au repos, gris foncé quand
//! elles sont désactivées, bleu d'accent pour l'outil actif.

mod atlas;
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
    BoneHead,
    Duplicate,
    Subdivide,
    Flip,
    XMirror,
    Plus,
    Scale,
    Transform,
    Globe,
    Local,
    Magnet,
}

const ALL: [Icon; 59] = [
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
    Icon::BoneHead,
    Icon::Duplicate,
    Icon::Subdivide,
    Icon::Flip,
    Icon::XMirror,
    Icon::Plus,
    Icon::Scale,
    Icon::Transform,
    Icon::Globe,
    Icon::Local,
    Icon::Magnet,
];

/// Toutes les icônes (aperçus, tests).
pub fn all() -> &'static [Icon] {
    &ALL
}

impl Icon {
    /// Document SVG complet de l'icône (glyphe blanc).
    pub fn svg(self) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">{}</svg>"#,
            svg::source(self)
        )
    }
}

/// Dessine `icon` dans le carré inscrit dans `rect`, de la couleur `tint`
/// (voir [`crate::theme::ICON`], [`crate::theme::ICON_DIM`],
/// [`crate::theme::ACCENT`]).
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, tint: Color32) {
    atlas::paint(painter, rect, icon, tint);
}

/// Prépare l'atlas d'icônes pour la densité de pixels courante (sinon il
/// est créé au premier dessin).
pub fn install(ctx: &egui::Context) {
    atlas::prepare(ctx);
}
