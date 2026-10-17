//! Police Inter (SIL Open Font License, voir `assets/fonts/OFL.txt`),
//! embarquée en deux graisses : Regular pour le texte courant, SemiBold pour
//! les titres et les libellés actifs. Les polices par défaut d'egui restent
//! en secours (symboles, emoji).

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

const REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
const SEMIBOLD_FAMILY: &str = "inter-semibold";

/// Installe Inter comme police proportionnelle et ajoute la famille
/// semi-grasse.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("inter".into(), Arc::new(FontData::from_static(REGULAR)));
    fonts
        .font_data
        .insert(SEMIBOLD_FAMILY.into(), Arc::new(FontData::from_static(SEMIBOLD)));

    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut regular = vec!["inter".to_owned()];
    regular.extend(fallback.iter().cloned());
    let mut bold = vec![SEMIBOLD_FAMILY.to_owned()];
    bold.extend(fallback);
    fonts.families.insert(FontFamily::Proportional, regular);
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD_FAMILY.into()), bold);
    ctx.set_fonts(fonts);
}

/// Police semi-grasse de la taille demandée (titres, libellés actifs).
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD_FAMILY.into()))
}
