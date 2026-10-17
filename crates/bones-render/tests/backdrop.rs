//! Flou d'arrière-plan (verre dépoli) rendu hors écran : un bord net devient
//! un dégradé large, une teinte unie reste unie, et seule la zone demandée
//! est lue dans la source.

use bones_render::{Backdrop, Region};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: [u32; 2] = [256, 128];

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// Source RGBA dont chaque pixel vaut `paint(x, y)`.
fn source(device: &wgpu::Device, queue: &wgpu::Queue, paint: impl Fn(u32, u32) -> u8) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("source"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut pixels = Vec::with_capacity((SIZE[0] * SIZE[1] * 4) as usize);
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let v = paint(x, y);
            pixels.extend_from_slice(&[v, v, v, 255]);
        }
    }
    queue.write_texture(
        texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE[0] * 4),
            rows_per_image: None,
        },
        texture.size(),
    );
    texture
}

/// Floute `region` de `src` et relit la sortie (rouge de chaque pixel).
fn blur(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    src: &wgpu::Texture,
    region: Region,
) -> (Vec<u8>, [u32; 2]) {
    let mut backdrop = Backdrop::new(device, FORMAT);
    let mut encoder = device.create_command_encoder(&Default::default());
    backdrop.blur(device, &mut encoder, src, &[region]);
    let out = backdrop.output_texture(0).expect("zone floutée");
    let [w, h] = [out.width(), out.height()];
    let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("relecture"),
        size: (row * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        out.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        out.size(),
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("lecture du tampon"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("attente du GPU");
    let data = slice.get_mapped_range().expect("tampon projeté");
    let reds = (0..h)
        .flat_map(|y| (0..w).map(move |x| (y, x)))
        .map(|(y, x)| data[(y * row + x * 4) as usize])
        .collect();
    (reds, [w, h])
}

#[test]
fn a_sharp_edge_becomes_a_wide_gradient() {
    let Some((device, queue)) = device() else {
        eprintln!("aucun adaptateur GPU : test du flou ignoré");
        return;
    };
    let src = source(&device, &queue, |x, _| if x < SIZE[0] / 2 { 0 } else { 255 });
    let full = Region {
        x: 0,
        y: 0,
        width: SIZE[0],
        height: SIZE[1],
    };
    let (out, [w, h]) = blur(&device, &queue, &src, full);
    assert_eq!([w, h], [SIZE[0] / 2, SIZE[1] / 2], "sortie à la demi-résolution");
    let mid = (h / 2 * w) as usize;
    let row = &out[mid..mid + w as usize];
    assert!(row[1] < 30 && row[w as usize - 2] > 225, "extrémités : {row:?}");
    // Le bord (au milieu) s'étale sur plusieurs pixels intermédiaires.
    let soft = row.iter().filter(|v| (40..=215).contains(*v)).count();
    assert!(soft >= 8, "transition trop nette ({soft} px) : {row:?}");
    assert!(
        row.windows(2).all(|p| i32::from(p[1]) + 2 >= i32::from(p[0])),
        "profil non monotone : {row:?}"
    );
}

#[test]
fn a_flat_colour_stays_flat_and_only_the_region_is_read() {
    let Some((device, queue)) = device() else {
        return;
    };
    // Gris 128 dans la moitié droite, blanc ailleurs : la zone ne couvre que
    // le gris, le blanc ne doit pas déborder dans le résultat.
    let src = source(&device, &queue, |x, _| if x >= 128 { 128 } else { 255 });
    let region = Region {
        x: 128,
        y: 16,
        width: 96,
        height: 64,
    };
    let (out, _) = blur(&device, &queue, &src, region);
    assert!(
        out.iter().all(|v| v.abs_diff(128) <= 2),
        "teinte altérée : {out:?}"
    );
}
