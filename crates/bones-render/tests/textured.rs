//! Rendu hors écran d'un carré texturé : la couleur de la texture doit
//! apparaître à l'image quand les textures sont affichées, et disparaître
//! (rendu « argile ») sinon. Un second carré sans coordonnées de texture
//! vérifie le repli sans tampon d'UV.

mod common;

use bones_core::mesh::{Material, Submesh, Texture};
use bones_core::{Mesh, OrbitCamera, Skin, Viewport};
use bones_render::{Renderer, Scene};
use common::{FORMAT, SIZE};
use glam::{Vec2, Vec3};

/// PNG 4×4 uni magenta.
fn magenta_png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 255, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

/// Carré 1×1 face à la caméra (plan XY), centré en (0, 1, 0).
fn quad(uvs: bool) -> Mesh {
    let mut mesh = Mesh {
        positions: vec![
            Vec3::new(-0.5, 0.5, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.5, 1.5, 0.0),
            Vec3::new(-0.5, 1.5, 0.0),
        ],
        normals: vec![Vec3::Z; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
        ..Default::default()
    };
    if uvs {
        mesh.uvs = vec![
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::ZERO,
        ];
    }
    mesh.materials = vec![Material {
        name: "magenta".into(),
        base_color: [1.0; 4],
        base_color_texture: Some(Texture::new("magenta.png", magenta_png())),
        ..Default::default()
    }];
    mesh.submeshes = vec![Submesh {
        indices: 0..6,
        material: Some(0),
    }];
    mesh
}

/// Nombre de pixels nettement magenta.
fn magenta_pixels(mesh: &Mesh, textured: bool) -> Option<usize> {
    let (device, queue) = common::device()?;
    let camera = OrbitCamera {
        yaw: 0.0,
        pitch: 0.0,
        ..Default::default()
    };
    let viewport = Viewport::new(SIZE[0] as f32, SIZE[1] as f32);
    let skin = Skin {
        weights: vec![Default::default(); mesh.vertex_count()],
    };
    let scene = Scene {
        view_proj: camera.view_proj(&viewport),
        eye: camera.eye(),
        viewport: [0.0, 0.0, viewport.width, viewport.height],
        background: [0.1, 0.1, 0.1],
        joints: &[],
        show_mesh: true,
        textured,
        weight_bone: None,
        lines: &[],
        bones: &[],
        overlay_lines: &[],
    };
    let mut renderer = Renderer::new(&device, FORMAT);
    renderer.set_mesh(&device, &queue, mesh, 1);
    renderer.set_skin(&device, &queue, &skin, 1);
    let data = common::render(&device, &queue, &mut renderer, &scene);
    common::dump(
        &data,
        SIZE,
        &format!("quad-textured-{textured}-uv-{}", mesh.has_uvs()),
    );
    // Rouge et bleu forts, vert faible.
    Some(
        common::pixels(&data)
            .iter()
            .filter(|p| p[2] > 120 && p[0] > 120 && p[1] < 60)
            .count(),
    )
}

#[test]
fn texture_color_reaches_the_frame() {
    let mesh = quad(true);
    let Some(textured) = magenta_pixels(&mesh, true) else {
        eprintln!("aucun adaptateur GPU : test de rendu ignoré");
        return;
    };
    let clay = magenta_pixels(&mesh, false).unwrap();
    let area = (SIZE[0] * SIZE[1]) as usize;
    assert!(
        textured > area / 50,
        "texture absente : {textured} pixels magenta"
    );
    assert_eq!(clay, 0, "les textures doivent être masquées en rendu argile");
}

#[test]
fn mesh_without_uvs_ignores_its_texture() {
    let Some(count) = magenta_pixels(&quad(false), true) else {
        eprintln!("aucun adaptateur GPU : test de rendu ignoré");
        return;
    };
    // Couleur de base blanche, pas de texture : aucun pixel magenta.
    assert_eq!(count, 0);
}
