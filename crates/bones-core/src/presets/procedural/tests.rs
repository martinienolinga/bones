use glam::Vec3;

use super::*;
use crate::presets::params::Params;
use crate::presets::roles::{Chain, Lr, detect};
use crate::{Clip, Rig, Skeleton, head_tail};

/// Têtes et queues monde de chaque os à chaque image.
pub(crate) fn frames(skeleton: &Skeleton, clip: &Clip) -> Vec<Vec<(Vec3, Vec3)>> {
    let mut pose = crate::Pose::rest(skeleton);
    (0..clip.frame_count)
        .map(|f| {
            clip.sample_into(f as f32, &mut pose);
            pose.world(skeleton)
                .iter()
                .zip(skeleton.bones())
                .map(|(w, b)| head_tail(w, b.length))
                .collect()
        })
        .collect()
}

/// Vérifie l'absence de NaN et, pour une boucle, que la dernière image
/// reprend la première (au déplacement horizontal de la racine près).
pub(crate) fn check_clip(skeleton: &Skeleton, clip: &Clip, looping: bool, name: &str) {
    let all = frames(skeleton, clip);
    for (f, frame) in all.iter().enumerate() {
        for (h, t) in frame {
            assert!(h.is_finite() && t.is_finite(), "{name}: NaN à l'image {f}");
        }
    }
    if looping {
        let (first, last) = (&all[0], &all[all.len() - 1]);
        let shift = last[0].0 - first[0].0;
        let shift = Vec3::new(shift.x, 0.0, shift.z);
        for (i, ((h0, t0), (h1, t1))) in first.iter().zip(last).enumerate() {
            let gap = (*h1 - shift - *h0).length().max((*t1 - shift - *t0).length());
            assert!(gap < 2e-3, "{name}: boucle discontinue sur l'os {i} ({gap})");
        }
    }
}

/// Pieds posés : un point au sol ne glisse pas d'une image à l'autre.
pub(crate) fn check_planted(skeleton: &Skeleton, roles: &RoleMap, clip: &Clip, name: &str) -> usize {
    let all = frames(skeleton, clip);
    let ground = roles.frame.ground;
    let mut contacts = 0;
    for side in Lr::BOTH {
        let Some(foot) = roles.bone(Chain::Leg(side), 2) else {
            continue;
        };
        let rest_ankle = skeleton.rest_head_tail(foot).0.y;
        for f in 1..all.len() {
            let (a0, b0) = all[f - 1][foot.index()];
            let (a1, b1) = all[f][foot.index()];
            if b0.y < ground + 0.003 && b1.y < ground + 0.003 {
                contacts += 1;
                let slide = (b1 - b0).length();
                assert!(
                    slide < 0.005,
                    "{name}: la plante glisse de {slide} m à l'image {f} ({side:?})"
                );
            }
            if a0.y < rest_ankle + 0.002 && a1.y < rest_ankle + 0.002 && b1.y < ground + 0.003 {
                let slide = (a1 - a0).length();
                assert!(
                    slide < 0.005,
                    "{name}: la cheville glisse de {slide} m à l'image {f} ({side:?})"
                );
            }
        }
    }
    contacts
}

fn walk(params: &[(&str, f32)], rig: &Rig) -> Clip {
    let roles = detect(&rig.skeleton);
    bake(
        Generator::HumanLocomotion,
        &Params::of(params),
        &rig.skeleton,
        &roles,
        30.0,
        1.0,
        2,
    )
    .unwrap()
}

#[test]
fn walk_on_demo_loops_and_plants_feet() {
    let rig = Rig::demo();
    let roles = detect(&rig.skeleton);
    for (name, params) in [
        ("walk", vec![("speed", 1.4)]),
        ("run", vec![("speed", 3.5), ("run", 1.0)]),
        ("sprint", vec![("speed", 7.0), ("run", 1.0)]),
        ("sneak", vec![("speed", 0.8), ("crouch", 1.0)]),
    ] {
        let clip = walk(&params, &rig);
        check_clip(&rig.skeleton, &clip, true, name);
        let contacts = check_planted(&rig.skeleton, &roles, &clip, name);
        assert!(contacts > 10, "{name}: {contacts} contacts");
    }
}

#[test]
fn every_builtin_preset_bakes_and_loops() {
    let demo = Rig::demo();
    for preset in crate::presets::builtin() {
        let crate::presets::PresetKind::Procedural(p) = &preset.kind else {
            continue;
        };
        let mut targets = vec![p.reference().skeleton()];
        if p.generator.rig_type() == crate::presets::RigType::Humanoid {
            targets.push(demo.skeleton.clone());
        }
        for skeleton in targets {
            let roles = detect(&skeleton);
            let clip = bake(p.generator, &p.params, &skeleton, &roles, 30.0, 1.0, 1)
                .unwrap_or_else(|e| panic!("{}: {e}", preset.info.id));
            assert!(clip.frame_count >= 2, "{}", preset.info.id);
            check_clip(&skeleton, &clip, preset.info.looping, &preset.info.id);
        }
        assert!(preset.info.frames > 0, "{}", preset.info.id);
    }
}
