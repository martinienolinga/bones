use std::sync::Arc;

use glam::{Vec2, Vec3};

use super::{temp_dir, tiny_png};
use crate::Rig;
use crate::io::{Format, Imported, export, import, obj};
use crate::mesh::{Material, Submesh, Texture};

#[test]
fn parses_quad_with_slashes_and_negative_indices() {
    let src = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 -2//1 -1//1\n";
    let mesh = obj::parse(src).unwrap();
    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    assert!(mesh.normals[0].abs_diff_eq(Vec3::Z, 1e-6));
}

#[test]
fn reports_bad_index() {
    assert!(obj::parse("v 0 0 0\nf 1 2 3\n").is_err());
}

#[test]
fn imports_mtl_and_texture_relative_to_model() {
    let dir = temp_dir("obj-mtl");
    std::fs::write(dir.join("tex.png"), tiny_png()).unwrap();
    std::fs::write(
        dir.join("model.mtl"),
        "newmtl rouge\nKd 1 0 0\nNs 250\nmap_Kd tex.png\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("model.obj"),
        "mtllib model.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nusemtl rouge\nf 1/1 2/2 3/3\n",
    )
    .unwrap();
    let Imported::Mesh(mesh) = import(&dir.join("model.obj")).unwrap() else {
        panic!("maillage attendu");
    };
    assert_eq!(mesh.materials.len(), 1);
    let m = &mesh.materials[0];
    assert_eq!(m.base_color, [1.0, 0.0, 0.0, 1.0]);
    let tex = m.base_color_texture.as_ref().expect("texture chargée");
    assert_eq!(&*tex.bytes, tiny_png().as_slice());
    assert_eq!(tex.mime, "image/png");
    assert_eq!(tex.decode_rgba8().unwrap().0, 2);
    assert!(mesh.has_uvs());
    // Origine des UV retournée (OBJ : en bas à gauche).
    assert!(mesh.uvs.contains(&Vec2::new(0.0, 1.0)));
    assert_eq!(mesh.parts()[0].material, Some(0));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Mannequin de démonstration avec UV, un matériau texturé et une partie.
pub(super) fn textured_demo() -> Rig {
    let mut rig = Rig::demo();
    let mesh = Arc::make_mut(&mut rig.mesh);
    mesh.uvs = mesh
        .positions
        .iter()
        .map(|p| Vec2::new(p.x.fract().abs(), p.y.fract().abs()))
        .collect();
    mesh.materials = vec![Material {
        name: "peau".into(),
        base_color: [0.9, 0.7, 0.6, 1.0],
        base_color_texture: Some(Texture::new("peau.png", tiny_png())),
        ..Default::default()
    }];
    mesh.submeshes = vec![Submesh {
        indices: 0..mesh.indices.len() as u32,
        material: Some(0),
    }];
    rig
}

#[test]
fn export_roundtrip_keeps_geometry_materials_and_textures() {
    let dir = temp_dir("obj-export");
    let rig = textured_demo();
    let files = export(&rig, Format::Obj, &dir.join("mannequin.obj")).unwrap();
    assert_eq!(files.len(), 3, "{files:?}");
    let Imported::Mesh(mesh) = import(&files[0]).unwrap() else {
        panic!("maillage attendu");
    };
    assert_eq!(mesh.vertex_count(), rig.mesh.vertex_count());
    assert_eq!(mesh.triangle_count(), rig.mesh.triangle_count());
    assert_eq!(mesh.materials.len(), 1);
    assert_eq!(
        &*mesh.materials[0].base_color_texture.as_ref().unwrap().bytes,
        tiny_png().as_slice()
    );
    // `tobj` renumérote les sommets : chaque couple (position, UV) importé
    // doit exister dans l'original.
    for (p, t) in mesh.positions.iter().zip(&mesh.uvs) {
        let found = rig
            .mesh
            .positions
            .iter()
            .zip(&rig.mesh.uvs)
            .any(|(q, u)| p.abs_diff_eq(*q, 1e-5) && t.abs_diff_eq(*u, 1e-5));
        assert!(found, "{p:?} {t:?}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
