//! Rendu hors écran du mannequin de démonstration : vérifie à l'exécution
//! que pipelines, MSAA et attachements transitoires passent la validation
//! wgpu, et que le maillage apparaît bien à l'image.
//!
//! Le test est ignoré (avec un message) si aucun adaptateur GPU n'est
//! disponible ; sous Linux, le pilote logiciel lavapipe
//! (`mesa-vulkan-drivers`) suffit.

use bones_core::{OrbitCamera, Rig, Viewport};
use bones_render::{BoneInstance, LineVertex, Renderer, Scene, bone_matrix};
use glam::{Vec3, Vec4};

const SIZE: [u32; 2] = [256, 192];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// Rend une image et retourne ses pixels BGRA.
fn render(device: &wgpu::Device, queue: &wgpu::Queue, samples: u32) -> Vec<u8> {
    let rig = Rig::demo();
    let mut renderer = Renderer::with_samples(device, FORMAT, samples);
    renderer.set_mesh(device, queue, &rig.mesh, 1);
    renderer.set_skin(device, queue, &rig.skin, 1);

    let mut camera = OrbitCamera::default();
    let (lo, hi) = rig.mesh.bounds().unwrap();
    camera.frame(lo, hi);
    let viewport = Viewport::new(SIZE[0] as f32, SIZE[1] as f32);
    let joints = rig.skinning_matrices();
    let bones: Vec<BoneInstance> = rig
        .world()
        .iter()
        .zip(rig.skeleton.bones())
        .map(|(w, b)| {
            BoneInstance::new(
                bone_matrix(w.to_matrix(), b.length),
                Vec4::new(1.0, 0.6, 0.1, 1.0),
            )
        })
        .collect();
    let grey = Vec4::new(0.5, 0.5, 0.5, 1.0);
    let lines = [
        LineVertex::new(Vec3::new(-2.0, 0.0, 0.0), grey),
        LineVertex::new(Vec3::new(2.0, 0.0, 0.0), grey),
    ];
    let scene = Scene {
        view_proj: camera.view_proj(&viewport),
        eye: camera.eye(),
        viewport: [0.0, 0.0, viewport.width, viewport.height],
        background: [0.1, 0.1, 0.1],
        joints: &joints,
        show_mesh: true,
        weight_bone: Some(0),
        lines: &lines,
        bones: &bones,
        overlay_lines: &lines,
    };

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (SIZE[0] * SIZE[1] * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Deux images de suite : la seconde réutilise tampons et attachements.
    for _ in 0..2 {
        renderer.prepare(device, queue, &scene);
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.render(device, &mut encoder, &view, SIZE);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE[0] * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: SIZE[0],
                height: SIZE[1],
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
    }

    readback.map_async(wgpu::MapMode::Read, .., |r| r.expect("lecture du tampon"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("attente GPU");
    readback.get_mapped_range(..).expect("tampon mappé").to_vec()
}

#[test]
fn renders_demo_rig_with_and_without_msaa() {
    let Some((device, queue)) = device() else {
        eprintln!("aucun adaptateur GPU : test de rendu ignoré");
        return;
    };
    for samples in [1, 4] {
        let pixels = render(&device, &queue, samples);
        let background = [26u8, 26, 26];
        let corner = &pixels[0..3];
        assert!(
            corner.iter().zip(background).all(|(a, b)| a.abs_diff(b) <= 1),
            "fond inattendu {corner:?} (MSAA {samples})"
        );
        let drawn = pixels
            .chunks_exact(4)
            .filter(|p| p[..3].iter().zip(background).any(|(a, b)| a.abs_diff(b) > 8))
            .count();
        let ratio = drawn as f32 / (SIZE[0] * SIZE[1]) as f32;
        // `BONES_RENDER_DUMP=dossier` : écrit les images (PPM) pour inspection.
        if let Ok(dir) = std::env::var("BONES_RENDER_DUMP") {
            let mut ppm = format!("P6 {} {} 255\n", SIZE[0], SIZE[1]).into_bytes();
            ppm.extend(pixels.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0]]));
            std::fs::write(format!("{dir}/demo-msaa{samples}.ppm"), ppm).unwrap();
        }
        assert!(
            ratio > 0.05,
            "presque rien n'est dessiné ({ratio}, MSAA {samples})"
        );
    }
}
