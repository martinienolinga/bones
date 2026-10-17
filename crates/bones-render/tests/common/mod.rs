//! Outils communs aux tests de rendu hors écran.
//!
//! Les tests sont ignorés (avec un message) si aucun adaptateur GPU n'est
//! disponible ; sous Linux, le pilote logiciel lavapipe
//! (`mesa-vulkan-drivers`) suffit.

// Chaque fichier de test n'utilise qu'une partie de ces outils.
#![allow(dead_code)]

use bones_render::{Readback, Renderer, Scene};

pub const SIZE: [u32; 2] = [256, 192];
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

pub fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// Rend `scene` deux fois de suite à l'écran (la seconde réutilise tampons
/// et attachements) et retourne les pixels RGBA de la dernière image.
pub fn render(device: &wgpu::Device, queue: &wgpu::Queue, renderer: &mut Renderer, scene: &Scene) -> Vec<u8> {
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
    let mut readback = None;
    for _ in 0..2 {
        renderer.prepare(device, queue, scene);
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.render(device, &mut encoder, &view, SIZE);
        readback = Readback::copy(device, &mut encoder, &target);
        queue.submit([encoder.finish()]);
    }
    readback.expect("format lisible").wait().expect("relecture GPU")
}

/// `BONES_RENDER_DUMP=dossier` : écrit l'image RGBA de `width` × `height`
/// pixels (PPM) pour inspection.
pub fn dump(pixels: &[u8], [width, height]: [u32; 2], name: &str) {
    if let Ok(dir) = std::env::var("BONES_RENDER_DUMP") {
        let mut ppm = format!("P6 {width} {height} 255\n").into_bytes();
        ppm.extend(pixels.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]));
        std::fs::write(format!("{dir}/{name}.ppm"), ppm).unwrap();
    }
}

/// Pixels RGBA de l'image.
pub fn pixels(data: &[u8]) -> &[[u8; 4]] {
    data.as_chunks::<4>().0
}
