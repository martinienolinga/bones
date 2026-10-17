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
