//! Ressources GPU du flou d'arrière-plan : les deux pipelines (réduction,
//! agrandissement) partagés, et la chaîne de textures propre à une zone.

use crate::blur::Region;

const SHADER: &str = include_str!("shaders/blur.wgsl");
/// Nombre de réductions (1/2 … 1/16) ; la sortie est à la demi-résolution.
const LEVELS: u32 = 4;

/// Pipelines, mise en page des liaisons et échantillonneur.
pub(crate) struct Kernels {
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
}

impl Kernels {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("flou"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flou"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(1, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("flou"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label, fs| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("flou"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            format,
            down: pipeline("flou-réduction", "fs_down"),
            up: pipeline("flou-agrandissement", "fs_up"),
            layout,
            sampler,
        }
    }

    fn bind(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flou"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn texture(&self, device: &wgpu::Device, size: [u32; 2], usage: wgpu::TextureUsages) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("flou"),
            size: wgpu::Extent3d {
                width: size[0].max(1),
                height: size[1].max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage,
            view_formats: &[],
        })
    }
}

/// Une passe : agrandissement ou réduction, source liée, cible.
struct Pass {
    up: bool,
    source: wgpu::BindGroup,
    target: wgpu::TextureView,
}

/// Textures d'une zone : copie pleine résolution, niveaux réduits, sortie.
pub(crate) struct Chain {
    size: [u32; 2],
    generation: u64,
    copy: wgpu::Texture,
    passes: Vec<Pass>,
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
}

impl Chain {
    pub fn new(device: &wgpu::Device, kernels: &Kernels, size: [u32; 2], generation: u64) -> Self {
        use wgpu::TextureUsages as U;
        let level = |k: u32| [(size[0] >> k).max(1), (size[1] >> k).max(1)];
        let copy = kernels.texture(device, size, U::COPY_DST | U::TEXTURE_BINDING);
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let mut source = view(&copy);
        let mut passes = Vec::new();
        // Réductions : 1/2, 1/4, 1/8, 1/16.
        for k in 1..=LEVELS {
            let target = view(&kernels.texture(device, level(k), U::RENDER_ATTACHMENT | U::TEXTURE_BINDING));
            passes.push(Pass {
                up: false,
                source: kernels.bind(device, &source),
                target: target.clone(),
            });
            source = target;
        }
        // Agrandissements : 1/8, 1/4, puis la sortie à 1/2.
        for k in (2..LEVELS).rev() {
            let target = view(&kernels.texture(device, level(k), U::RENDER_ATTACHMENT | U::TEXTURE_BINDING));
            passes.push(Pass {
                up: true,
                source: kernels.bind(device, &source),
                target: target.clone(),
            });
            source = target;
        }
        let output = kernels.texture(
            device,
            level(1),
            U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::COPY_SRC,
        );
        let output_view = view(&output);
        passes.push(Pass {
            up: true,
            source: kernels.bind(device, &source),
            target: output_view.clone(),
        });
        Self {
            size,
            generation,
            copy,
            passes,
            output,
            output_view,
        }
    }

    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.output_view
    }

    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.output
    }

    /// Copie la zone depuis `source`, puis enchaîne les passes de flou.
    pub fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        kernels: &Kernels,
        source: &wgpu::Texture,
        region: Region,
    ) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: source,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            self.copy.as_image_copy(),
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        for pass in &self.passes {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("flou"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &pass.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(if pass.up { &kernels.up } else { &kernels.down });
            rp.set_bind_group(0, &pass.source, &[]);
            rp.draw(0..3, 0..1);
        }
    }
}
