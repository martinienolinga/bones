use super::*;

#[test]
fn capsule_is_closed_and_outward() {
    let m = Mesh::capsule(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), 0.5, 16, 4);
    let (lo, hi) = m.bounds().unwrap();
    assert!(lo.abs_diff_eq(Vec3::new(-0.5, -0.5, -0.5), 1e-4));
    assert!(hi.abs_diff_eq(Vec3::new(0.5, 0.5, 2.5), 1e-4));
    // Normales sortantes : elles s'éloignent de l'axe de la capsule.
    let center = Vec3::new(0.0, 0.0, 1.0);
    let outward = m
        .positions
        .iter()
        .zip(&m.normals)
        .filter(|(p, n)| (**p - center).dot(**n) > 0.0)
        .count();
    assert!(outward as f32 > 0.95 * m.vertex_count() as f32);
}

#[test]
fn primitives_are_consistent_and_face_outward() {
    for p in Primitive::ALL {
        let m = p.mesh();
        let n = m.vertex_count();
        assert!(m.triangle_count() > 0, "{p:?}");
        assert_eq!(m.normals.len(), n, "{p:?}");
        assert!(m.has_uvs(), "{p:?}");
        assert!(m.indices.iter().all(|i| (*i as usize) < n), "{p:?}");
        let (lo, hi) = m.bounds().unwrap();
        assert!(lo.cmpge(Vec3::splat(-0.5 - 1e-5)).all() && hi.cmple(Vec3::splat(0.5 + 1e-5)).all());
        // Sens direct : la normale géométrique de chaque triangle non
        // dégénéré suit les normales de ses sommets.
        for t in m.indices.as_chunks::<3>().0 {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            let g = (b - a).cross(c - a);
            if g.length() < 1e-7 {
                continue;
            }
            let vn: Vec3 = t.iter().map(|i| m.normals[*i as usize]).sum();
            assert!(g.dot(vn) > 0.0, "{p:?} : triangle {t:?} retourné");
        }
    }
}
