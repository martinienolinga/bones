use glam::Vec3;

use crate::io::{Format, Imported, bvh, import_bytes};
use crate::{Rig, head_tail};

const SAMPLE: &str = "HIERARCHY
ROOT Hips
{
  OFFSET 0 0 0
  CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
  JOINT Spine
  {
    OFFSET 0 10 0
    CHANNELS 3 Zrotation Xrotation Yrotation
    End Site
    {
      OFFSET 0 10 0
    }
  }
  JOINT Leg
  {
    OFFSET 5 0 0
    CHANNELS 3 Zrotation Xrotation Yrotation
    End Site
    {
      OFFSET 0 -40 0
    }
  }
}
MOTION
Frames: 2
Frame Time: 0.04
0 90 0 0 0 0 0 0 0 0 0 0
0 90 0 0 0 0 90 0 0 0 0 0
";

fn rig_of(bytes: &[u8]) -> Rig {
    match import_bytes(Format::Bvh, bytes, "mocap", None).unwrap() {
        Imported::Rig(rig) => rig,
        Imported::Mesh(_) => panic!("squelette attendu"),
    }
}

#[test]
fn imports_hierarchy_and_motion() {
    let rig = rig_of(SAMPLE.as_bytes());
    assert_eq!(rig.skeleton.len(), 3);
    assert_eq!(rig.clip.keys().len(), 2);
    assert!((rig.clip.fps - 25.0).abs() < 1e-3);
    // Hauteur 60 unités (> 50) : centimètres → mètres ; hanches à 0,9 m.
    let mut pose = rig.pose.clone();
    rig.clip.sample_into(1.0, &mut pose);
    let w = pose.world(&rig.skeleton);
    let spine = rig.skeleton.find("Spine").unwrap();
    let (head, tail) = head_tail(&w[spine.index()], rig.skeleton.bone(spine).length);
    assert!(head.abs_diff_eq(Vec3::new(0.0, 1.0, 0.0), 1e-4), "{head:?}");
    // Rotation de 90° autour de Z : la colonne pointe vers −X.
    assert!(tail.abs_diff_eq(Vec3::new(-0.1, 1.0, 0.0), 1e-4), "{tail:?}");
}

#[test]
fn export_roundtrip_preserves_joint_positions() {
    let mut rig = Rig::demo();
    let pose = rig.pose.clone();
    rig.clip_mut().set_key(0, pose);
    let mut moved = rig.pose.clone();
    moved.bones[0].translation.y += 0.2;
    moved.bones[3].rotation = glam::Quat::from_rotation_z(0.6);
    moved.bones[8].rotation = glam::Quat::from_rotation_x(-0.4);
    rig.clip_mut().set_key(10, moved);

    let text = bvh::export(&rig).unwrap();
    let back = rig_of(text.as_bytes());
    assert_eq!(back.skeleton.len(), rig.skeleton.len());
    for frame in [0.0, 4.0, 10.0] {
        let (mut a, mut b) = (rig.pose.clone(), back.pose.clone());
        rig.clip.sample_into(frame, &mut a);
        back.clip.sample_into(frame, &mut b);
        let (wa, wb) = (a.world(&rig.skeleton), b.world(&back.skeleton));
        for bone in rig.skeleton.ids() {
            let other = back.skeleton.find(&rig.skeleton.bone(bone).name).unwrap();
            let (ha, hb) = (wa[bone.index()].translation, wb[other.index()].translation);
            assert!(
                ha.abs_diff_eq(hb, 1e-3),
                "{} image {frame}: {ha:?} / {hb:?}",
                rig.skeleton.bone(bone).name
            );
        }
    }
}
