//! FBX témoin construit avec l'écrivain de `fbxcel` : un triangle texturé
//! en Z-up, deux os, une peau et une rotation animée.

use fbxcel::low::FbxVersion;
use fbxcel::low::v7400::AttributeValue as A;
use fbxcel::tree::v7400::{NodeId, Tree};
use fbxcel::writer::v7400::binary::{FbxFooter, Writer};
use glam::Vec3;

use super::tiny_png;
use crate::io::{Format, Imported, import_bytes};
use crate::{Rig, head_tail};

const TICKS: i64 = 46_186_158_000;

fn s(v: &str) -> A {
    A::String(v.to_owned())
}

fn node(tree: &mut Tree, parent: NodeId, name: &str, attrs: Vec<A>) -> NodeId {
    let id = tree.append_new(parent, name);
    for a in attrs {
        tree.append_attribute(id, a);
    }
    id
}

fn props(tree: &mut Tree, parent: NodeId, list: Vec<(&str, Vec<A>)>) {
    let p70 = node(tree, parent, "Properties70", vec![]);
    for (name, values) in list {
        let mut attrs = vec![s(name), s(""), s(""), s("A")];
        attrs.extend(values);
        node(tree, p70, "P", attrs);
    }
}

fn object(tree: &mut Tree, objects: NodeId, class: &str, id: i64, name: &str, sub: &str) -> NodeId {
    node(
        tree,
        objects,
        class,
        vec![A::I64(id), s(&format!("{name}\0\u{1}{class}")), s(sub)],
    )
}

fn sample_fbx() -> Vec<u8> {
    let mut t = Tree::default();
    let root = t.root().node_id();
    let gs = node(&mut t, root, "GlobalSettings", vec![]);
    let i = |v: i32| vec![A::I32(v)];
    props(
        &mut t,
        gs,
        vec![
            ("UpAxis", i(2)),
            ("UpAxisSign", i(1)),
            ("FrontAxis", i(1)),
            ("FrontAxisSign", i(-1)),
            ("CoordAxis", i(0)),
            ("CoordAxisSign", i(1)),
            ("UnitScaleFactor", vec![A::F64(100.0)]),
            ("TimeMode", i(6)),
        ],
    );
    let o = node(&mut t, root, "Objects", vec![]);
    let geo = object(&mut t, o, "Geometry", 10, "Tri", "Mesh");
    node(
        &mut t,
        geo,
        "Vertices",
        vec![A::ArrF64(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 2.0])],
    );
    node(&mut t, geo, "PolygonVertexIndex", vec![A::ArrI32(vec![0, 1, -3])]);
    let uv = node(&mut t, geo, "LayerElementUV", vec![A::I32(0)]);
    node(&mut t, uv, "MappingInformationType", vec![s("ByPolygonVertex")]);
    node(&mut t, uv, "ReferenceInformationType", vec![s("IndexToDirect")]);
    node(
        &mut t,
        uv,
        "UV",
        vec![A::ArrF64(vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0])],
    );
    node(&mut t, uv, "UVIndex", vec![A::ArrI32(vec![0, 1, 2])]);
    let lm = node(&mut t, geo, "LayerElementMaterial", vec![A::I32(0)]);
    node(&mut t, lm, "MappingInformationType", vec![s("AllSame")]);
    node(&mut t, lm, "ReferenceInformationType", vec![s("IndexToDirect")]);
    node(&mut t, lm, "Materials", vec![A::ArrI32(vec![0])]);
    object(&mut t, o, "Model", 20, "Tri", "Mesh");
    let mat = object(&mut t, o, "Material", 30, "Rouge", "");
    props(
        &mut t,
        mat,
        vec![("DiffuseColor", vec![A::F64(1.0), A::F64(0.0), A::F64(0.0)])],
    );
    object(&mut t, o, "Texture", 40, "Peau", "");
    let video = object(&mut t, o, "Video", 41, "Peau", "Clip");
    node(&mut t, video, "Content", vec![A::Binary(tiny_png())]);
    object(&mut t, o, "Model", 50, "Racine", "LimbNode");
    let tip = object(&mut t, o, "Model", 51, "Bout", "LimbNode");
    props(
        &mut t,
        tip,
        vec![("Lcl Translation", vec![A::F64(0.0), A::F64(0.0), A::F64(1.0)])],
    );
    object(&mut t, o, "Deformer", 60, "Peau", "Skin");
    let identity = || A::ArrF64(glam::DMat4::IDENTITY.to_cols_array().to_vec());
    let tip_link = A::ArrF64(
        glam::DMat4::from_translation(glam::DVec3::Z)
            .to_cols_array()
            .to_vec(),
    );
    for (id, points, link) in [(61, vec![0, 1], identity()), (62, vec![2], tip_link)] {
        let c = object(&mut t, o, "Deformer", id, "Groupe", "Cluster");
        let n = points.len();
        node(&mut t, c, "Indexes", vec![A::ArrI32(points)]);
        node(&mut t, c, "Weights", vec![A::ArrF64(vec![1.0; n])]);
        node(&mut t, c, "Transform", vec![identity()]);
        node(&mut t, c, "TransformLink", vec![link]);
    }
    object(&mut t, o, "AnimationStack", 70, "Prise", "");
    object(&mut t, o, "AnimationLayer", 71, "Base", "");
    object(&mut t, o, "AnimationCurveNode", 72, "R", "");
    let curve = object(&mut t, o, "AnimationCurve", 73, "", "");
    node(&mut t, curve, "KeyTime", vec![A::ArrI64(vec![0, TICKS])]);
    node(&mut t, curve, "KeyValueFloat", vec![A::ArrF32(vec![0.0, 90.0])]);

    let c = node(&mut t, root, "Connections", vec![]);
    let oo = [
        (20, 0),
        (10, 20),
        (30, 20),
        (41, 40),
        (50, 0),
        (51, 50),
        (60, 10),
        (61, 60),
        (62, 60),
        (50, 61),
        (51, 62),
        (71, 70),
        (72, 71),
    ];
    for (child, parent) in oo {
        node(&mut t, c, "C", vec![s("OO"), A::I64(child), A::I64(parent)]);
    }
    for (child, parent, prop) in [
        (40, 30, "DiffuseColor"),
        (72, 50, "Lcl Rotation"),
        (73, 72, "d|X"),
    ] {
        node(
            &mut t,
            c,
            "C",
            vec![s("OP"), A::I64(child), A::I64(parent), s(prop)],
        );
    }

    let mut writer = Writer::new(std::io::Cursor::new(Vec::new()), FbxVersion::V7_4).unwrap();
    writer.write_tree(&t).unwrap();
    writer
        .finalize_and_flush(&FbxFooter::default())
        .unwrap()
        .into_inner()
}

fn tip_head(rig: &Rig, frame: f32) -> Vec3 {
    let mut pose = rig.pose.clone();
    rig.clip.sample_into(frame, &mut pose);
    let w = pose.world(&rig.skeleton);
    let tip = rig.skeleton.find("Bout").unwrap();
    head_tail(&w[tip.index()], rig.skeleton.bone(tip).length).0
}

#[test]
fn imports_mesh_texture_skin_and_animation() {
    let Imported::Rig(rig) = import_bytes(Format::Fbx, &sample_fbx(), "fbx", None).unwrap() else {
        panic!("document attendu");
    };
    let mesh = &rig.mesh;
    assert_eq!(mesh.triangle_count(), 1);
    // Z-up → Y-up : le sommet (0, 0, 2) monte en (0, 2, 0).
    assert!(
        mesh.positions
            .iter()
            .any(|p| p.abs_diff_eq(Vec3::new(0.0, 2.0, 0.0), 1e-5)),
        "{:?}",
        mesh.positions
    );
    assert!(mesh.has_uvs());
    assert_eq!(mesh.materials.len(), 1);
    assert_eq!(mesh.materials[0].base_color[0], 1.0);
    assert_eq!(
        &*mesh.materials[0].base_color_texture.as_ref().unwrap().bytes,
        tiny_png().as_slice()
    );
    assert_eq!(mesh.parts()[0].material, Some(0));

    assert_eq!(rig.skeleton.len(), 2);
    assert!(!rig.needs_bind());
    let top = mesh.positions.iter().position(|p| p.y > 1.5).unwrap();
    let tip = rig.skeleton.find("Bout").unwrap();
    assert!((rig.skin.weights[top].weight_of(tip) - 1.0).abs() < 1e-3);

    assert_eq!(rig.clip.keys().len(), 31);
    assert!(tip_head(&rig, 0.0).abs_diff_eq(Vec3::new(0.0, 1.0, 0.0), 1e-4));
    // 90° autour de X (fichier) : le bout passe devant, en +Z.
    let end = tip_head(&rig, 30.0);
    assert!(end.abs_diff_eq(Vec3::new(0.0, 0.0, 1.0), 1e-3), "{end:?}");
}

#[test]
fn rejects_ascii_fbx() {
    assert!(import_bytes(Format::Fbx, b"; FBX 7.4.0 project file", "a", None).is_err());
}

#[test]
fn read_animations_lists_named_clips() {
    let (skeleton, clips) = crate::io::read_animations(Format::Fbx, &sample_fbx(), None).unwrap();
    assert_eq!(skeleton.len(), 2);
    assert_eq!(clips.len(), 1, "une pile d'animation");
    assert_eq!(clips[0].0, "Prise");
    assert_eq!(clips[0].1.keys().len(), 31);
}
