//! Outils communs aux tests de rendu hors écran.
//!
//! Les tests sont ignorés (avec un message) si aucun adaptateur GPU n'est
//! disponible ; sous Linux, le pilote logiciel lavapipe
//! (`mesa-vulkan-drivers`) suffit.

use bones_render::{Renderer, Scene};

pub const SIZE: [u32; 2] = [256, 192];
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

pub fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// Rend `scene` deux fois de suite (la seconde réutilise tampons et
/// attachements) et retourne les pixels BGRA de la dernière image.
pub fn render(device: &wgpu::Device, queue: &wgpu::Queue, renderer: &mut Renderer, scene: &Scene) -> Vec<u8> {
    let extent = wgpu::Extent3d {
        width: SIZE[0],
        height: SIZE[1],
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: extent,
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
    for _ in 0..2 {
        renderer.prepare(device, queue, scene);
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
            extent,
        );
        queue.submit([encoder.finish()]);
    }
    readback.map_async(wgpu::MapMode::Read, .., |r| r.expect("lecture du tampon"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("attente GPU");
    readback.get_mapped_range(..).expect("tampon mappé").to_vec()
}

/// `BONES_RENDER_DUMP=dossier` : écrit l'image (PPM) pour inspection.
pub fn dump(pixels: &[u8], name: &str) {
    if let Ok(dir) = std::env::var("BONES_RENDER_DUMP") {
        let mut ppm = format!("P6 {} {} 255\n", SIZE[0], SIZE[1]).into_bytes();
        ppm.extend(pixels.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0]]));
        std::fs::write(format!("{dir}/{name}.ppm"), ppm).unwrap();
    }
}

/// Pixels BGRA de l'image.
pub fn pixels(data: &[u8]) -> &[[u8; 4]] {
    data.as_chunks::<4>().0
}
