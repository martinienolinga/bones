//! Familles de teintes des icônes et enveloppe SVG commune.
//!
//! Les icônes sont plates : uniquement des aplats. Chaque fragment SVG
//! (`viewBox` 32 × 32) écrit ses couleurs avec trois jetons que l'enveloppe
//! remplace par les teintes de la famille :
//! - `@p` : teinte de base (formes principales) ;
//! - `@s` : teinte claire (formes secondaires) ;
//! - `@d` : teinte foncée (faces d'ombre, rainures).

/// Teintes d'une famille d'outils.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hue {
    pub base: &'static str,
    pub soft: &'static str,
    pub deep: &'static str,
}

/// Os et rigging : orange.
pub const RIG: Hue = Hue {
    base: "#FF9F0A",
    soft: "#FFCC70",
    deep: "#C76F00",
};
/// Déplacement et IK : bleu.
pub const MOVE: Hue = Hue {
    base: "#0A84FF",
    soft: "#70B7FF",
    deep: "#0060C7",
};
/// Rotation et FK : violet.
pub const ROTATE: Hue = Hue {
    base: "#BF5AF2",
    soft: "#DCA5F8",
    deep: "#8E35BD",
};
/// Épingles et validation : vert.
pub const PIN: Hue = Hue {
    base: "#30D158",
    soft: "#8BE6A1",
    deep: "#1F9B3F",
};
/// Suppression : rouge.
pub const DANGER: Hue = Hue {
    base: "#FF453A",
    soft: "#FF9A93",
    deep: "#B8261D",
};
/// Animation et clés : jaune.
pub const ANIM: Hue = Hue {
    base: "#FFD60A",
    soft: "#FFE88A",
    deep: "#C9A300",
};
/// Physique (balistique, centre de masse) : sarcelle.
pub const PHYSICS: Hue = Hue {
    base: "#2ED3B7",
    soft: "#90EADB",
    deep: "#1C9E89",
};
/// Fichiers (galerie, import, export) : bleu ciel.
pub const FILES: Hue = Hue {
    base: "#5AC8FA",
    soft: "#ADE4FD",
    deep: "#2B95C9",
};
/// Affichage (œil, grille, fantômes) : indigo.
pub const VIEW: Hue = Hue {
    base: "#7D7AFF",
    soft: "#BDBBFF",
    deep: "#5552D6",
};
/// Commandes générales (annuler, réglages) : gris clair.
pub const NEUTRAL: Hue = Hue {
    base: "#E5E5EA",
    soft: "#A1A1A8",
    deep: "#6E6E75",
};

/// Document SVG complet d'une icône, jetons de teinte remplacés.
pub fn document(hue: Hue, body: &str) -> String {
    let body = body
        .replace("@p", hue.base)
        .replace("@s", hue.soft)
        .replace("@d", hue.deep);
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">{body}</svg>"#)
}
