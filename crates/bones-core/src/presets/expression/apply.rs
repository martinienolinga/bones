//! Application d'une expression faciale à un squelette : détection des os du
//! visage (noms canoniques, puis correspondances étrangères), construction de
//! la pose du visage (intensité, miroir), et tête de référence pour l'aperçu.
//!
//! Tout est pur (sans état). Le miroir gauche ↔ droite réutilise
//! [`crate::rig::mirror::mirrored_pose`] (reflet en espace monde, qui respecte
//! les bases de repos propres à chaque os), restreint aux os du visage : le
//! corps n'est jamais touché.

use glam::Vec3;

use super::data::{ExpressionData, ExpressionReport, scaled};
use super::mapping;
use crate::autorig::face::{self, FaceBone, FaceLayout, FaceMarkers};
use crate::rig::mirror::mirrored_pose;
use crate::{BoneId, Pose, Skeleton};

/// Os du visage présents dans `skeleton`, par nom canonique
/// ([`face::present`]) puis par correspondance étrangère ([`mapping::foreign`])
/// pour les rigs importés. Un rôle déjà trouvé n'est pas réattribué.
pub fn present_mapped(skeleton: &Skeleton) -> Vec<(FaceBone, BoneId)> {
    let mut found = face::present(skeleton);
    let mut seen: Vec<FaceBone> = found.iter().map(|(b, _)| *b).collect();
    for id in skeleton.ids() {
        let name = &skeleton.bone(id).name;
        if FaceBone::from_name(name).is_some() {
            continue; // nom canonique : déjà traité par face::present
        }
        if let Some(fb) = mapping::foreign(name)
            && !seen.contains(&fb)
        {
            seen.push(fb);
            found.push((fb, id));
        }
    }
    found
}

/// Pose du visage produite par l'expression sur `skeleton` : repos + deltas
/// mis à l'échelle (`intensity`), décalages adaptés à la taille du visage,
/// éventuellement retournée gauche ↔ droite.
/// Seuls les os du visage présents sont posés ; tout le reste reste au repos.
/// Retourne aussi la couverture (os appliqués / ignorés).
pub fn face_pose(
    data: &ExpressionData,
    skeleton: &Skeleton,
    intensity: f32,
    mirror: bool,
) -> (Pose, ExpressionReport) {
    let present = present_mapped(skeleton);
    let size = face_scale(skeleton, &present);
    let mut pose = Pose::rest(skeleton);
    let mut report = ExpressionReport::default();
    for (name, delta) in &data.bones {
        match present.iter().find(|(b, _)| b.name() == name) {
            Some((_, id)) => {
                let mut t = scaled(delta, intensity);
                t.translation *= size;
                pose.bones[id.index()] = t;
                report.applied.push(name.clone());
            }
            None => report.skipped.push(name.clone()),
        }
    }
    if mirror {
        pose = mirrored_pose(skeleton, &pose);
    }
    (pose, report)
}

/// Rayon des yeux de la tête de référence (mètres) : les décalages des
/// expressions sont écrits à cette échelle.
const REFERENCE_EYE_RADIUS: f32 = 0.013;

/// Taille du visage de `skeleton` par rapport à la tête de référence,
/// d'après la longueur des paupières (proportionnelle au rayon de l'œil,
/// voir [`FaceLayout`]) : un modèle en centimètres reçoit des décalages
/// cent fois plus grands. 1 si le modèle n'a pas de paupières.
fn face_scale(skeleton: &Skeleton, present: &[(FaceBone, BoneId)]) -> f32 {
    let reference = 1.3 * REFERENCE_EYE_RADIUS;
    present
        .iter()
        .find(|(b, _)| matches!(b, FaceBone::EyelidTopL | FaceBone::EyelidTopR))
        .map(|(_, id)| skeleton.bone(*id).length / reference)
        .filter(|s| s.is_finite() && *s > 0.0)
        .map_or(1.0, |s| s.clamp(1e-3, 1e3))
}

/// Tête de référence autonome (`head.x` + os du visage, avec dents et langue)
/// bâtie avec des marqueurs synthétiques, sans maillage : aperçu des
/// expressions dans la galerie et tests. Suit les conventions de l'auto-rig.
pub fn reference_head() -> Skeleton {
    let mut s = Skeleton::new();
    s.add_bone_world(
        "head.x",
        None,
        Vec3::new(0.0, 1.46, 0.0),
        Vec3::new(0.0, 1.75, 0.0),
    );
    face::build(&mut s, &FaceLayout::new(reference_markers(), 0.0));
    s
}

/// Marqueurs d'un visage humain de référence (centré sur x = 0), dents et
/// langue comprises pour couvrir les visèmes.
fn reference_markers() -> FaceMarkers {
    FaceMarkers {
        eye: Vec3::new(0.035, 1.62, 0.085),
        eye_radius: REFERENCE_EYE_RADIUS,
        brow: Vec3::new(0.035, 1.66, 0.09),
        nose_tip: Vec3::new(0.0, 1.60, 0.12),
        lip_top: Vec3::new(0.0, 1.565, 0.11),
        lip_bottom: Vec3::new(0.0, 1.55, 0.11),
        mouth_corner: Vec3::new(0.025, 1.558, 0.10),
        chin: Vec3::new(0.0, 1.53, 0.105),
        jaw: Vec3::new(0.0, 1.61, 0.0),
        ear: Some(Vec3::new(0.085, 1.63, -0.02)),
        teeth_top: Some(Vec3::new(0.0, 1.562, 0.095)),
        teeth_bottom: Some(Vec3::new(0.0, 1.552, 0.095)),
        tongue: Some([Vec3::new(0.0, 1.55, 0.06), Vec3::new(0.0, 1.552, 0.1)]),
    }
}
