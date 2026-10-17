//! Shaders, disposition des liaisons et pipelines de rendu.

use crate::layouts::{LINE_ATTRS, bone_buffers, mesh_buffers};
use crate::scene::LineVertex;

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Sources WGSL, assemblées en un seul module.
const SHADER: &str = concat!(
    include_str!("shaders/common.wgsl"),
    include_str!("shaders/mesh.wgsl"),
    include_str!("shaders/bone.wgsl"),
    include_str!("shaders/line.wgsl"),
);

pub(crate) struct Pipelines {
    pub bind_layout: wgpu::BindGroupLayout,
    /// Maillage skinné, opaque.
    pub mesh: wgpu::RenderPipeline,
    /// Os, fantômes et poignées (instanciés, transparence possible).
    pub bone: wgpu::RenderPipeline,
    /// Lignes testées en profondeur (grille).
    pub line: wgpu::RenderPipeline,
    /// Lignes toujours visibles (trajectoires, repères).
    pub overlay_line: wgpu::RenderPipeline,
}

/// Paramètres variables d'un pipeline.
struct Spec<'a> {
    label: &'a str,
    vs: &'a str,
    fs: &'a str,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    topology: wgpu::PrimitiveTopology,
    cull: Option<wgpu::Face>,
    blend: Option<wgpu::BlendState>,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bones"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_layout = bind_group_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bones"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let create = |s: Spec| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(s.label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(s.vs),
                    compilation_options: Default::default(),
                    buffers: s.buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: s.topology,
                    cull_mode: s.cull,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(s.depth_write),
                    depth_compare: Some(s.depth_compare),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(s.fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: s.blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };

        let blend = Some(wgpu::BlendState::ALPHA_BLENDING);
        let lines = [Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &LINE_ATTRS,
        })];
        let line = |label, depth_compare| Spec {
            label,
            vs: "vs_line",
            fs: "fs_line",
            buffers: &lines,
            topology: wgpu::PrimitiveTopology::LineList,
            cull: None,
            blend,
            depth_write: false,
            depth_compare,
        };

        Self {
            mesh: create(Spec {
                label: "mesh",
                vs: "vs_mesh",
                fs: "fs_mesh",
                buffers: &mesh_buffers(),
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull: None,
                blend: None,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
            }),
            bone: create(Spec {
                label: "bones",
                vs: "vs_bone",
                fs: "fs_bone",
                buffers: &bone_buffers(),
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull: Some(wgpu::Face::Back),
                blend,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
            }),
            line: create(line("lines", wgpu::CompareFunction::LessEqual)),
            overlay_line: create(line("overlay-lines", wgpu::CompareFunction::Always)),
            bind_layout,
        }
    }

    /// Groupe de liaison : uniformes globaux et matrices des os.
    pub fn bind_group(
        &self,
        device: &wgpu::Device,
        globals: &wgpu::Buffer,
        joints: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: joints.as_entire_binding(),
                },
            ],
        })
    }
}

fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let buffer = |binding, visibility, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("globals"),
        entries: &[
            buffer(
                0,
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                wgpu::BufferBindingType::Uniform,
            ),
            buffer(
                1,
                wgpu::ShaderStages::VERTEX,
                wgpu::BufferBindingType::Storage { read_only: true },
            ),
        ],
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn shaders_are_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(super::SHADER).expect("WGSL invalide");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("validation naga");
        for entry in ["vs_mesh", "fs_mesh", "vs_bone", "fs_bone", "vs_line", "fs_line"] {
            assert!(
                module.entry_points.iter().any(|e| e.name == entry),
                "{entry} manquant"
            );
        }
    }
}
