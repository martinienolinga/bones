//! Valeurs non finies (NaN, infini) : le JSON ne sait pas les représenter
//! (serde_json écrit `null`, illisible ensuite). Elles sont remplacées avant
//! l'écriture, pour qu'une valeur aberrante ne rende jamais tout un projet
//! illisible.

use std::sync::Arc;

use glam::Vec3;

use crate::{Clip, Material, Rig, Transform};

/// Corrige la partie JSON d'un document (le maillage n'y a plus que ses
/// matériaux ; les poses des clés sont écrites à part). Appelée avant
/// l'écriture, et après la lecture pour les fichiers venus d'ailleurs.
pub(super) fn make_finite(rig: &mut Rig) {
    for id in rig.skeleton.ids() {
        let bone = rig.skeleton.bone_mut(id);
        if !transform_ok(&bone.rest) {
            bone.rest = Transform::IDENTITY;
        }
        if !bone.length.is_finite() {
            bone.length = 0.1;
        }
    }
    for t in &mut rig.pose.bones {
        if !transform_ok(t) {
            *t = Transform::IDENTITY;
        }
    }
    for limb in &mut rig.limbs {
        for p in [&mut limb.target, &mut limb.pole] {
            if !p.is_finite() {
                *p = Vec3::ZERO;
            }
        }
    }
    rig.pins.retain(|p| p.target.is_finite());
    if !rig.clip.fps.is_finite() {
        Arc::make_mut(&mut rig.clip).fps = Clip::default().fps;
    }
    let fresh = Material::default();
    if rig.mesh.materials.iter().any(|m| !material_ok(m)) {
        for m in &mut Arc::make_mut(&mut rig.mesh).materials {
            if !m.base_color.iter().all(|c| c.is_finite()) {
                m.base_color = fresh.base_color;
            }
            if !m.metallic.is_finite() {
                m.metallic = fresh.metallic;
            }
            if !m.roughness.is_finite() {
                m.roughness = fresh.roughness;
            }
        }
    }
}

fn transform_ok(t: &Transform) -> bool {
    t.translation.is_finite() && t.rotation.is_finite() && t.scale.is_finite()
}

fn material_ok(m: &Material) -> bool {
    m.base_color.iter().all(|c| c.is_finite()) && m.metallic.is_finite() && m.roughness.is_finite()
}
