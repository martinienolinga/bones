use glam::Vec3;

use super::obj::textured_demo;
use crate::io::{Format, Imported, import_bytes, ply};

#[test]
fn ascii_quad_with_uvs() {
    let src = "ply\nformat ascii 1.0\ncomment test\nelement vertex 4\nproperty float x\nproperty float y\n\
               property float z\nproperty float s\nproperty float t\nelement face 1\n\
               property list uchar int vertex_indices\nend_header\n\
               0 0 0 0 0\n1 0 0 1 0\n1 1 0 1 1\n0 1 0 0 1\n4 0 1 2 3\n";
    let Imported::Mesh(mesh) = import_bytes(Format::Ply, src.as_bytes(), "q", None).unwrap() else {
        panic!("maillage attendu");
    };
    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    assert!(mesh.has_uvs());
    assert_eq!(mesh.uvs[0], glam::Vec2::new(0.0, 1.0));
    assert!(mesh.normals[0].abs_diff_eq(Vec3::Z, 1e-6));
}

#[test]
fn binary_roundtrip() {
    let rig = textured_demo();
    let bytes = ply::export(&rig);
    let Imported::Mesh(mesh) = import_bytes(Format::Ply, &bytes, "m", None).unwrap() else {
        panic!("maillage attendu");
    };
    assert_eq!(mesh.vertex_count(), rig.mesh.vertex_count());
    assert_eq!(mesh.indices, rig.mesh.indices);
    for (a, b) in mesh.positions.iter().zip(&rig.mesh.positions) {
        assert!(a.abs_diff_eq(*b, 1e-6));
    }
    for (a, b) in mesh.uvs.iter().zip(&rig.mesh.uvs) {
        assert!(a.abs_diff_eq(*b, 1e-6));
    }
}

#[test]
fn rejects_truncated_data() {
    let src = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nend_header\n0 1\n";
    assert!(import_bytes(Format::Ply, src.as_bytes(), "x", None).is_err());
}
