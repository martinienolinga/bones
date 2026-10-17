use glam::Vec3;

use super::visibility::Occluder;
use super::*;

#[test]
fn occluder_blocks_segments_crossing_the_surface() {
    // Carré de deux triangles dans le plan z = 0, plus un sommet isolé.
    let mesh = Mesh {
        positions: vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        ..Default::default()
    };
    let occluder = Occluder::new(&mesh);
    let (front, back) = (Vec3::new(0.2, 0.1, 1.0), Vec3::new(0.2, 0.1, -1.0));
    assert!(!occluder.visible(u32::MAX, front, back));
    assert!(occluder.visible(u32::MAX, front, Vec3::new(3.0, 0.0, -1.0)));
    assert!(occluder.visible(u32::MAX, front, front + Vec3::X));
    // Les triangles du sommet de départ ne cachent rien.
    assert!(occluder.visible(0, mesh.positions[0], Vec3::new(-0.5, -0.5, -1.0)));
}

#[test]
fn heat_ignores_a_close_bone_hidden_behind_another_part() {
    // Un « tronc » épais et un « bras » mince tout proche, en deux pièces :
    // le flanc du tronc est plus près de l'os du bras que du sien, mais ne
    // le voit pas (la surface du bras est entre les deux).
    let mut mesh = Mesh::capsule(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 0.3, 24, 16);
    let trunk_vertices = mesh.positions.len();
    mesh.append(&Mesh::capsule(
        Vec3::new(0.45, 0.0, 0.0),
        Vec3::new(0.45, 1.0, 0.0),
        0.06,
        12,
        16,
    ));
    let mut s = Skeleton::new();
    let trunk = s.add_bone_world("trunk", None, Vec3::ZERO, Vec3::Y);
    let arm = s.add_bone_world(
        "arm",
        Some(trunk),
        Vec3::new(0.45, 0.0, 0.0),
        Vec3::new(0.45, 1.0, 0.0),
    );
    let skin = heat_weights(&mesh, &s);
    for (v, vw) in skin.weights.iter().enumerate() {
        let (own, other) = if v < trunk_vertices {
            (trunk, arm)
        } else {
            (arm, trunk)
        };
        assert!(vw.weight_of(own) > 0.99, "sommet {v} : {vw:?}");
        assert!(vw.weight_of(other) < 0.01);
    }
}
