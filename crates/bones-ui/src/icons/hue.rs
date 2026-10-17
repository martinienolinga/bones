//! Familles de teintes des icônes et enveloppe SVG commune.
//!
//! Chaque icône est un fragment SVG (`viewBox` 32 × 32) qui utilise trois
//! dégradés fournis par l'enveloppe :
//! - `url(#a)` : dégradé principal de la famille, du clair (haut) au foncé
//!   (bas), en coordonnées de l'icône pour que toutes les formes partagent la
//!   même lumière ;
//! - `url(#b)` : variante diagonale plus lumineuse (faces secondaires) ;
//! - `url(#h)` : reflet blanc qui s'estompe vers le bas (brillance).

/// Teintes d'une famille d'outils.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hue {
    pub light: &'static str,
    pub dark: &'static str,
    pub glow: &'static str,
}

/// Os et rigging : ambre → orange.
pub const RIG: Hue = Hue {
    light: "#FFD86E",
    dark: "#FF7417",
    glow: "#FFF3C9",
};
/// Déplacement et IK : cyan → bleu.
pub const MOVE: Hue = Hue {
    light: "#72EAFF",
    dark: "#1D63FF",
    glow: "#DDF9FF",
};
/// Rotation et FK : violet → magenta.
pub const ROTATE: Hue = Hue {
    light: "#CFA8FF",
    dark: "#C42DF0",
    glow: "#F4E9FF",
};
/// Épingles et validation : vert.
pub const PIN: Hue = Hue {
    light: "#86F7AE",
    dark: "#0FA35A",
    glow: "#E2FFEC",
};
/// Suppression : rouge → corail.
pub const DANGER: Hue = Hue {
    light: "#FF9A8F",
    dark: "#EE2A45",
    glow: "#FFE4E0",
};
/// Animation et clés : or.
pub const ANIM: Hue = Hue {
    light: "#FFE680",
    dark: "#F09A00",
    glow: "#FFF8D6",
};
/// Physique (balistique, centre de masse) : sarcelle.
pub const PHYSICS: Hue = Hue {
    light: "#66F4DA",
    dark: "#0B9690",
    glow: "#D9FFF7",
};
/// Fichiers (galerie, import, export) : bleu ciel.
pub const FILES: Hue = Hue {
    light: "#93D4FF",
    dark: "#2A78FF",
    glow: "#E6F5FF",
};
/// Affichage (œil, grille, fantômes) : indigo → lilas.
pub const VIEW: Hue = Hue {
    light: "#BCC0FF",
    dark: "#6A55F0",
    glow: "#EEEDFF",
};
/// Commandes générales (annuler, réglages) : blanc nacré.
pub const NEUTRAL: Hue = Hue {
    light: "#FFFFFF",
    dark: "#B4BAC6",
    glow: "#FFFFFF",
};

/// Document SVG complet d'une icône.
pub fn document(hue: Hue, body: &str) -> String {
    let Hue { light, dark, glow } = hue;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><defs><linearGradient id="a" gradientUnits="userSpaceOnUse" x1="0" y1="3" x2="0" y2="29"><stop offset="0" stop-color="{light}"/><stop offset="1" stop-color="{dark}"/></linearGradient><linearGradient id="b" gradientUnits="userSpaceOnUse" x1="4" y1="4" x2="28" y2="28"><stop offset="0" stop-color="{glow}"/><stop offset="1" stop-color="{light}"/></linearGradient><linearGradient id="h" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".6"/><stop offset=".55" stop-color="#fff" stop-opacity="0"/></linearGradient></defs>{body}</svg>"##
    )
}
