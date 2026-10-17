use glam::Vec3;

use super::names::{Part, parse};
use super::*;
use crate::presets::RefRig;
use crate::{Rig, Skeleton};

fn names(skeleton: &Skeleton, bones: &[crate::BoneId]) -> Vec<String> {
    bones.iter().map(|b| skeleton.bone(*b).name.clone()).collect()
}

fn chain_names(skeleton: &Skeleton, map: &RoleMap, chain: Chain) -> Vec<String> {
    names(skeleton, &map.chain(chain))
}

/// Renomme les os d'après une table (ancien → nouveau).
fn renamed(mut skeleton: Skeleton, table: &[(&str, &str)]) -> Skeleton {
    for (old, new) in table {
        let id = skeleton.find(old).unwrap_or_else(|| panic!("os {old}"));
        skeleton.bone_mut(id).name = (*new).to_owned();
    }
    skeleton
}

#[test]
fn tokens_split_conventions() {
    assert_eq!(names::tokens("mixamorig:LeftUpLeg"), ["left", "up", "leg"]);
    assert_eq!(names::tokens("LHipJoint"), ["l", "hip", "joint"]);
    assert_eq!(
        names::tokens("DEF-upper_arm.L.001"),
        ["def", "upper", "arm", "l", "001"]
    );
    assert_eq!(names::tokens("Bip01 R Forearm"), ["bip", "01", "r", "forearm"]);
}

#[test]
fn parse_reads_side_and_part() {
    let p = parse("mixamorig:RightForeArm");
    assert_eq!((p.side, p.part), (Some(Lr::Right), Some(Part::Arm(Some(1)))));
    assert_eq!(parse("thigh.l").part, Some(Part::Leg(Some(0))));
    assert_eq!(parse("calf_r").side, Some(Lr::Right));
    assert_eq!(parse("upperarm_twist_01_l").part, Some(Part::Ignore));
    assert_eq!(parse("HeadTop_End").part, Some(Part::End));
    assert_eq!(parse("LeftHandIndex2").part, Some(Part::Finger(1)));
    assert_eq!(parse("Bip01 Pelvis").part, Some(Part::Hips));
    assert_eq!(parse("LHipJoint").part, Some(Part::Ignore));
    assert_eq!(parse("b_LeftLeg01_015").part, Some(Part::Leg(None)));
}

fn assert_humanoid(skeleton: &Skeleton, map: &RoleMap, root: &str) {
    assert_eq!(map.rig_type, RigType::Humanoid);
    assert_eq!(names(skeleton, &[map.root().unwrap()]), [root]);
    assert_eq!(chain_names(skeleton, map, Chain::Leg(Lr::Left)).len(), 4);
    assert_eq!(chain_names(skeleton, map, Chain::Arm(Lr::Right)).len(), 3);
    assert_eq!(chain_names(skeleton, map, Chain::Clavicle(Lr::Left)).len(), 1);
    assert_eq!(chain_names(skeleton, map, Chain::Head).len(), 1);
    assert!(
        map.frame.forward.abs_diff_eq(Vec3::Z, 1e-3),
        "{:?}",
        map.frame.forward
    );
}

#[test]
fn demo_rig_is_humanoid() {
    let rig = Rig::demo();
    let map = detect(&rig.skeleton);
    assert_eq!(map.rig_type, RigType::Humanoid);
    let s = &rig.skeleton;
    assert_eq!(names(s, &[map.root().unwrap()]), ["root.x"]);
    assert_eq!(
        chain_names(s, &map, Chain::Leg(Lr::Left)),
        ["thigh.l", "leg.l", "foot.l", "toes.l"]
    );
    assert_eq!(
        chain_names(s, &map, Chain::Arm(Lr::Right)),
        ["arm.r", "forearm.r", "hand.r"]
    );
    assert_eq!(chain_names(s, &map, Chain::Clavicle(Lr::Left)), ["shoulder.l"]);
    assert_eq!(chain_names(s, &map, Chain::Spine).len(), 3);
    assert_eq!(chain_names(s, &map, Chain::Head), ["head.x"]);
    assert_eq!(chain_names(s, &map, Chain::Finger(Lr::Left, 0)).len(), 3);
    assert!(map.frame.forward.abs_diff_eq(Vec3::Z, 1e-3));
    assert!((map.frame.size - 0.9).abs() < 0.2, "{}", map.frame.size);
}

const MIXAMO: &[(&str, &str)] = &[
    ("root.x", "mixamorig:Hips"),
    ("spine_01.x", "mixamorig:Spine"),
    ("spine_02.x", "mixamorig:Spine1"),
    ("spine_03.x", "mixamorig:Spine2"),
    ("neck.x", "mixamorig:Neck"),
    ("head.x", "mixamorig:Head"),
    ("shoulder.l", "mixamorig:LeftShoulder"),
    ("arm.l", "mixamorig:LeftArm"),
    ("forearm.l", "mixamorig:LeftForeArm"),
    ("hand.l", "mixamorig:LeftHand"),
    ("shoulder.r", "mixamorig:RightShoulder"),
    ("arm.r", "mixamorig:RightArm"),
    ("forearm.r", "mixamorig:RightForeArm"),
    ("hand.r", "mixamorig:RightHand"),
    ("thigh.l", "mixamorig:LeftUpLeg"),
    ("leg.l", "mixamorig:LeftLeg"),
    ("foot.l", "mixamorig:LeftFoot"),
    ("toes.l", "mixamorig:LeftToeBase"),
    ("thigh.r", "mixamorig:RightUpLeg"),
    ("leg.r", "mixamorig:RightLeg"),
    ("foot.r", "mixamorig:RightFoot"),
    ("toes.r", "mixamorig:RightToeBase"),
];

#[test]
fn mixamo_names() {
    let mut s = renamed(RefRig::Humanoid.skeleton(), MIXAMO);
    let head = s.find("mixamorig:Head").unwrap();
    let tail = s.rest_head_tail(head).1;
    s.add_bone_world("mixamorig:HeadTop_End", Some(head), tail, tail + Vec3::Y * 0.05);
    let map = detect(&s);
    assert_humanoid(&s, &map, "mixamorig:Hips");
    assert_eq!(
        chain_names(&s, &map, Chain::Leg(Lr::Left)),
        [
            "mixamorig:LeftUpLeg",
            "mixamorig:LeftLeg",
            "mixamorig:LeftFoot",
            "mixamorig:LeftToeBase"
        ]
    );
    let end = s.find("mixamorig:HeadTop_End").unwrap();
    assert_eq!(map.role(end), None);
}

#[test]
fn unreal_names_with_ground_root_and_twist_bones() {
    let table: Vec<(&str, &str)> = MIXAMO
        .iter()
        .map(|(ours, _)| {
            let unreal = match *ours {
                "root.x" => "pelvis",
                "spine_01.x" => "spine_01",
                "spine_02.x" => "spine_02",
                "spine_03.x" => "spine_03",
                "neck.x" => "neck_01",
                "head.x" => "head",
                o => {
                    let (base, side) = o.split_once('.').unwrap();
                    let base = match base {
                        "shoulder" => "clavicle",
                        "arm" => "upperarm",
                        "forearm" => "lowerarm",
                        "leg" => "calf",
                        "toes" => "ball",
                        b => b,
                    };
                    Box::leak(format!("{base}_{side}").into_boxed_str())
                }
            };
            (*ours, unreal)
        })
        .collect();
    let mut s = renamed(RefRig::Humanoid.skeleton(), &table);
    // Os « root » au sol, parent du bassin : on reconstruit un squelette.
    let mut with_root = Skeleton::new();
    let ground = with_root.add_bone_world("root", None, Vec3::ZERO, Vec3::Z * 0.2);
    let world = s.rest_world();
    let mut ids = Vec::new();
    for (i, b) in s.bones().iter().enumerate() {
        let (h, t) = crate::head_tail(&world[i], b.length);
        let parent = b.parent().map(|p| ids[p.index()]).or(Some(ground));
        ids.push(with_root.add_bone_world(b.name.clone(), parent, h, t));
    }
    let upper = with_root.find("upperarm_l").unwrap();
    let (h, t) = with_root.rest_head_tail(upper);
    with_root.add_bone_world("upperarm_twist_01_l", Some(upper), h.lerp(t, 0.5), t);
    s = with_root;
    let map = detect(&s);
    assert_eq!(map.rig_type, RigType::Humanoid);
    assert_eq!(names(&s, &[map.root().unwrap()]), ["pelvis"]);
    assert_eq!(names(&s, &[map.bone(Chain::Ground, 0).unwrap()]), ["root"]);
    assert_eq!(
        chain_names(&s, &map, Chain::Arm(Lr::Left)),
        ["upperarm_l", "lowerarm_l", "hand_l"]
    );
    assert_eq!(
        chain_names(&s, &map, Chain::Leg(Lr::Right)),
        ["thigh_r", "calf_r", "foot_r", "ball_r"]
    );
    assert_eq!(map.role(s.find("upperarm_twist_01_l").unwrap()), None);
}

/// Squelette BVH à la CMU (noms et os d'attache des hanches).
fn cmu() -> Skeleton {
    use crate::presets::reference::Builder;
    let mut b = Builder::new();
    let hips = b.bone("Hips", None, [0.0, 0.95, 0.0], [0.0, 1.05, 0.0]);
    let back = b.bone("LowerBack", Some(hips), [0.0, 1.05, 0.0], [0.0, 1.2, 0.0]);
    let spine = b.bone("Spine", Some(back), [0.0, 1.2, 0.0], [0.0, 1.35, 0.0]);
    let spine1 = b.bone("Spine1", Some(spine), [0.0, 1.35, 0.0], [0.0, 1.48, 0.0]);
    let neck = b.bone("Neck", Some(spine1), [0.0, 1.48, 0.0], [0.0, 1.55, 0.0]);
    let neck1 = b.bone("Neck1", Some(neck), [0.0, 1.55, 0.0], [0.0, 1.6, 0.0]);
    b.bone("Head", Some(neck1), [0.0, 1.6, 0.0], [0.0, 1.78, 0.0]);
    for (s, x) in [("Left", 1.0f32), ("Right", -1.0)] {
        let c = &s[..1];
        let p = |v: [f32; 3]| [v[0] * x, v[1], v[2]];
        let hj = b.bone(
            &format!("{c}HipJoint"),
            Some(hips),
            p([0.0, 0.95, 0.0]),
            p([0.1, 0.9, 0.0]),
        );
        let up = b.bone(
            &format!("{s}UpLeg"),
            Some(hj),
            p([0.1, 0.9, 0.0]),
            p([0.1, 0.5, 0.02]),
        );
        let leg = b.bone(
            &format!("{s}Leg"),
            Some(up),
            p([0.1, 0.5, 0.02]),
            p([0.1, 0.08, 0.0]),
        );
        let foot = b.bone(
            &format!("{s}Foot"),
            Some(leg),
            p([0.1, 0.08, 0.0]),
            p([0.1, 0.02, 0.13]),
        );
        b.bone(
            &format!("{s}ToeBase"),
            Some(foot),
            p([0.1, 0.02, 0.13]),
            p([0.1, 0.01, 0.18]),
        );
        let sh = b.bone(
            &format!("{s}Shoulder"),
            Some(spine1),
            p([0.02, 1.45, 0.0]),
            p([0.18, 1.45, 0.0]),
        );
        let arm = b.bone(
            &format!("{s}Arm"),
            Some(sh),
            p([0.18, 1.45, 0.0]),
            p([0.45, 1.45, 0.0]),
        );
        let fore = b.bone(
            &format!("{s}ForeArm"),
            Some(arm),
            p([0.45, 1.45, 0.0]),
            p([0.7, 1.45, 0.0]),
        );
        let hand = b.bone(
            &format!("{s}Hand"),
            Some(fore),
            p([0.7, 1.45, 0.0]),
            p([0.78, 1.45, 0.0]),
        );
        b.bone(
            &format!("{c}Thumb"),
            Some(hand),
            p([0.72, 1.45, 0.02]),
            p([0.76, 1.45, 0.05]),
        );
    }
    b.0
}

#[test]
fn cmu_bvh_names() {
    let s = cmu();
    let map = detect(&s);
    assert_eq!(map.rig_type, RigType::Humanoid);
    assert_eq!(names(&s, &[map.root().unwrap()]), ["Hips"]);
    assert_eq!(
        chain_names(&s, &map, Chain::Spine),
        ["LowerBack", "Spine", "Spine1"]
    );
    assert_eq!(chain_names(&s, &map, Chain::Neck), ["Neck", "Neck1"]);
    assert_eq!(
        chain_names(&s, &map, Chain::Leg(Lr::Left)),
        ["LeftUpLeg", "LeftLeg", "LeftFoot", "LeftToeBase"]
    );
    assert_eq!(
        chain_names(&s, &map, Chain::Arm(Lr::Right)),
        ["RightArm", "RightForeArm", "RightHand"]
    );
    assert_eq!(map.role(s.find("LHipJoint").unwrap()), None);
}

#[test]
fn reference_rigs_have_expected_types() {
    let expected = [
        (RefRig::Humanoid, RigType::Humanoid),
        (RefRig::Quadruped, RigType::Quadruped),
        (RefRig::Bird, RigType::Bird),
        (RefRig::Fish, RigType::Fish),
        (RefRig::Serpent, RigType::Serpent),
        (RefRig::Spider, RigType::MultiLegged),
        (RefRig::Insect, RigType::MultiLegged),
        (RefRig::Tree, RigType::Plant),
        (RefRig::Grass, RigType::Plant),
        (RefRig::Flower, RigType::Plant),
        (RefRig::Palm, RigType::Plant),
        (RefRig::Rope, RigType::Chain),
    ];
    for (rig, ty) in expected {
        let map = detect(&rig.skeleton());
        assert_eq!(map.rig_type, ty, "{rig:?}");
        assert!(map.root().is_some(), "{rig:?}");
    }
}

#[test]
fn quadruped_roles_and_frame() {
    let s = RefRig::Quadruped.skeleton();
    let map = detect(&s);
    assert_eq!(
        chain_names(&s, &map, Chain::Arm(Lr::Left)),
        ["arm.l", "forearm.l", "hand.l", "toes_front.l"]
    );
    assert_eq!(
        chain_names(&s, &map, Chain::Leg(Lr::Right)),
        ["thigh.r", "leg.r", "foot.r", "toes.r"]
    );
    assert_eq!(chain_names(&s, &map, Chain::Tail).len(), 3);
    assert!(map.frame.forward.abs_diff_eq(Vec3::Z, 1e-3));
}

/// Sans noms exploitables, la topologie retrouve membres, côtés et colonne.
#[test]
fn anonymous_quadruped_by_topology() {
    let mut s = RefRig::Quadruped.skeleton();
    let original = s.clone();
    for (i, b) in s.ids().collect::<Vec<_>>().into_iter().enumerate() {
        s.bone_mut(b).name = format!("Bone.{i:03}");
    }
    let map = detect(&s);
    assert_eq!(map.rig_type, RigType::Quadruped);
    let by_name = |chain| chain_names(&original, &map, chain);
    assert_eq!(
        by_name(Chain::Arm(Lr::Left)),
        ["arm.l", "forearm.l", "hand.l", "toes_front.l"]
    );
    assert_eq!(
        by_name(Chain::Leg(Lr::Right)),
        ["thigh.r", "leg.r", "foot.r", "toes.r"]
    );
    assert_eq!(by_name(Chain::Root), ["root.x"]);
    assert_eq!(by_name(Chain::Head), ["head.x"]);
    assert_eq!(by_name(Chain::Tail).len(), 3);
}

#[test]
fn anonymous_humanoid_by_topology() {
    let mut s = RefRig::Humanoid.skeleton();
    let original = s.clone();
    for (i, b) in s.ids().collect::<Vec<_>>().into_iter().enumerate() {
        s.bone_mut(b).name = format!("joint{i}");
    }
    let map = detect(&s);
    assert_eq!(map.rig_type, RigType::Humanoid);
    let by_name = |chain| chain_names(&original, &map, chain);
    assert_eq!(
        by_name(Chain::Leg(Lr::Left)),
        ["thigh.l", "leg.l", "foot.l", "toes.l"]
    );
    assert_eq!(
        by_name(Chain::Arm(Lr::Right))[..3],
        ["arm.r", "forearm.r", "hand.r"]
    );
    assert_eq!(by_name(Chain::Clavicle(Lr::Left)), ["shoulder.l"]);
    assert_eq!(by_name(Chain::Head), ["head.x"]);
}

#[test]
fn spider_legs_are_numbered_front_to_back() {
    let s = RefRig::Spider.skeleton();
    let map = detect(&s);
    assert_eq!(map.rig_type, RigType::MultiLegged);
    for pair in 0..4u8 {
        let left = chain_names(&s, &map, Chain::LegPair(pair, Lr::Left));
        assert_eq!(
            left.first().map(String::as_str),
            Some(format!("leg{}_01.l", pair + 1).as_str())
        );
    }
}

#[test]
fn plants_get_branch_chains() {
    let s = RefRig::Tree.skeleton();
    let map = detect(&s);
    let free: Vec<_> = s
        .ids()
        .filter(|b| map.role(*b).is_none())
        .map(|b| s.bone(b).name.clone())
        .collect();
    assert!(free.is_empty(), "{free:?}");
    assert_eq!(chain_names(&s, &map, Chain::Root), ["trunk_01"]);
    assert_eq!(chain_names(&s, &map, Chain::Branch(0))[0], "trunk_02");
}

#[test]
fn manual_override_moves_a_role() {
    let s = RefRig::Humanoid.skeleton();
    let mut map = detect(&s);
    let head = s.find("head.x").unwrap();
    let neck = s.find("neck.x").unwrap();
    map.set(neck, Some(Role::new(Chain::Head, 0)));
    assert_eq!(map.bone(Chain::Head, 0), Some(neck));
    assert_eq!(map.role(head), None);
}
