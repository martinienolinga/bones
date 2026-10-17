use crate::Rig;
use crate::io::{Format, Imported, import_bytes, stl};

#[test]
fn ascii_stl_is_converted_to_y_up() {
    let src = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
    let Imported::Mesh(mesh) = import_bytes(Format::Stl, src.as_bytes(), "t", None).unwrap() else {
        panic!("maillage attendu");
    };
    assert_eq!(mesh.triangle_count(), 1);
    // (0, 1, 0) en Z-up devient (0, 0, -1) en Y-up.
    assert!(
        mesh.positions
            .iter()
            .any(|p| p.abs_diff_eq(glam::Vec3::new(0.0, 0.0, -1.0), 1e-6))
    );
}

#[test]
fn binary_roundtrip_in_millimetres() {
    let rig = Rig::demo();
    let bytes = stl::export(&rig);
    let Imported::Mesh(mesh) = import_bytes(Format::Stl, &bytes, "m", None).unwrap() else {
        panic!("maillage attendu");
    };
    assert_eq!(mesh.triangle_count(), rig.mesh.triangle_count());
    let (lo, hi) = mesh.bounds().unwrap();
    let (rlo, rhi) = rig.mesh.bounds().unwrap();
    assert!(
        lo.abs_diff_eq(rlo, 1e-3) && hi.abs_diff_eq(rhi, 1e-3),
        "{lo:?} {hi:?}"
    );
}
