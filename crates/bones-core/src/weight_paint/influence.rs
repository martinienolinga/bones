//! Copie de travail flottante des poids d'un sommet : liste d'influences en
//! `f32` (sans perte de quantification pendant un trait), limite à quatre os
//! et normalisation automatique des os déformants, comme Blender.

use crate::{BoneId, VertexWeights};

/// Influences d'un sommet en flottant ; chaque poids est dans `[0, 1]`, la
/// somme est quelconque (elle n'est ramenée à 1 qu'à la quantification).
pub(crate) type Influences = Vec<(BoneId, f32)>;

/// Décode les influences quantifiées d'un sommet.
pub(crate) fn decode(vw: &VertexWeights) -> Influences {
    vw.iter().collect()
}

/// Quantifie des influences pour le stockage GPU (somme ramenée à 1, quatre
/// os au plus) via [`VertexWeights::from_influences`].
pub(crate) fn encode(infl: &Influences) -> VertexWeights {
    VertexWeights::from_influences(infl.clone())
}

pub(crate) fn weight_of(infl: &Influences, bone: BoneId) -> f32 {
    infl.iter().find(|(b, _)| *b == bone).map_or(0.0, |(_, w)| *w)
}

/// Fixe le poids d'un os : met à jour, ajoute, ou retire s'il devient nul.
pub(crate) fn set_weight(infl: &mut Influences, bone: BoneId, w: f32) {
    let w = w.clamp(0.0, 1.0);
    match infl.iter_mut().find(|(b, _)| *b == bone) {
        Some(e) => e.1 = w,
        None if w > 0.0 => infl.push((bone, w)),
        None => {}
    }
    infl.retain(|(_, w)| *w > 1e-6);
}

/// Au plus `max` influences : les plus faibles sont évincées, mais jamais `keep`.
pub(crate) fn limit(infl: &mut Influences, max: usize, keep: Option<BoneId>) {
    while infl.len() > max.max(1) {
        let victim = infl
            .iter()
            .enumerate()
            .filter(|(_, (b, _))| Some(*b) != keep)
            .min_by(|a, b| a.1.1.total_cmp(&b.1.1))
            .map(|(i, _)| i);
        let Some(i) = victim else { break };
        infl.remove(i);
    }
}

/// Garde le poids de `fixed` tel quel et met les autres os déformants à
/// l'échelle pour que la somme des poids déformants vaille 1 (« Auto
/// Normalize » de Blender). Les os de contrôle (non déformants) sont ignorés.
pub(crate) fn auto_normalize(infl: &mut Influences, fixed: BoneId, deform: &[bool]) {
    let is_deform = |b: BoneId| deform.get(b.index()).copied().unwrap_or(true);
    let locked = weight_of(infl, fixed);
    let others: f32 = infl
        .iter()
        .filter(|(b, _)| *b != fixed && is_deform(*b))
        .map(|(_, w)| *w)
        .sum();
    let budget = (1.0 - locked).max(0.0);
    if others > 1e-8 {
        let scale = budget / others;
        for (b, w) in infl.iter_mut() {
            if *b != fixed && is_deform(*b) {
                *w *= scale;
            }
        }
    }
    infl.retain(|(_, w)| *w > 1e-6);
}

/// Normalise tous les os déformants d'un sommet à une somme de 1 (sans os
/// privilégié) : « Normalize All ».
pub(crate) fn normalize_deform(infl: &mut Influences, deform: &[bool]) {
    let is_deform = |b: BoneId| deform.get(b.index()).copied().unwrap_or(true);
    let total: f32 = infl.iter().filter(|(b, _)| is_deform(*b)).map(|(_, w)| *w).sum();
    if total > 1e-8 {
        let scale = 1.0 / total;
        for (b, w) in infl.iter_mut() {
            if is_deform(*b) {
                *w *= scale;
            }
        }
    }
    infl.retain(|(_, w)| *w > 1e-6);
}

/// Drapeaux « os déformant » indexés par os.
pub(crate) fn deform_flags(skeleton: &crate::Skeleton) -> Vec<bool> {
    skeleton.ids().map(|id| skeleton.bone(id).deform).collect()
}
