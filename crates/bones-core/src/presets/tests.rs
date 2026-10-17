//! Tests transversaux des préréglages et planches-contacts.

use crate::presets::params::Params;
use crate::presets::procedural::{Generator, bake};
use crate::presets::roles::detect;
use crate::presets::sheet::sheet;
use crate::{Rig, Skeleton};

fn save_sheet(dir: &str, name: &str, skeleton: &Skeleton, clip: &crate::Clip) {
    let roles = detect(skeleton);
    let img = sheet(skeleton, &roles, clip, 10);
    img.save(format!("{dir}/{name}.png")).unwrap();
}

/// Planches-contacts de tous les préréglages intégrés, sur leur squelette
/// de référence (et sur le mannequin pour les humains) :
/// `BONES_PRESET_SHEETS=dossier`.
#[test]
#[ignore]
fn sheets() {
    let Ok(dir) = std::env::var("BONES_PRESET_SHEETS") else {
        return;
    };
    let rig = Rig::demo();
    for preset in crate::presets::builtin() {
        let crate::presets::PresetKind::Procedural(p) = &preset.kind else {
            continue;
        };
        let skeleton = if p.generator.rig_type() == crate::presets::RigType::Humanoid {
            rig.skeleton.clone()
        } else {
            p.reference().skeleton()
        };
        let roles = detect(&skeleton);
        let clip = bake(p.generator, &p.params, &skeleton, &roles, 30.0, 1.0, 1).unwrap();
        let name = preset.info.id.trim_start_matches("builtin.");
        save_sheet(&dir, name, &skeleton, &clip);
    }
}

#[test]
fn debug_demo_arm() {
    let rig = Rig::demo();
    let s = &rig.skeleton;
    let roles = detect(s);
    for name in [
        "shoulder.l",
        "arm.l",
        "forearm.l",
        "hand.l",
        "index1.l",
        "pinky1.l",
        "thumb1.l",
        "middle1.l",
        "thigh.l",
        "leg.l",
        "foot.l",
        "toes.l",
    ] {
        if let Some(b) = s.find(name) {
            let (h, t) = s.rest_head_tail(b);
            println!("{name}: {h:.3?} -> {t:.3?} role {:?}", roles.role(b));
        }
    }
    println!("frame {:?}", roles.frame);
}

#[test]
fn debug_walk_arm() {
    let rig = Rig::demo();
    let s = &rig.skeleton;
    let roles = detect(s);
    let clip = bake(
        Generator::HumanLocomotion,
        &Params::of(&[("speed", 1.4)]),
        s,
        &roles,
        30.0,
        1.0,
        1,
    )
    .unwrap();
    let mut pose = crate::Pose::rest(s);
    clip.sample_into(0.0, &mut pose);
    let w = pose.world(s);
    for name in ["shoulder.r", "arm.r", "forearm.r", "hand.r"] {
        let b = s.find(name).unwrap();
        let (h, t) = crate::head_tail(&w[b.index()], s.bone(b).length);
        println!("{name}: {h:.3?} -> {t:.3?} dir {:.3?}", (t - h).normalize());
    }
    let root = s.find("root.x").unwrap();
    let heights: Vec<f32> = (0..clip.frame_count)
        .map(|f| {
            clip.sample_into(f as f32, &mut pose);
            pose.world(s)[root.index()].translation.y
        })
        .collect();
    println!("root heights {heights:.3?} rest {}", s.rest_head_tail(root).0.y);
}
