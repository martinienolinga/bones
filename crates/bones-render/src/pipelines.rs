//! Shaders, disposition des liaisons et pipelines de rendu.

use crate::bindings;
use crate::layouts::{LINE_ATTRS, bone_buffers, mesh_buffers, object_buffers};
use crate::materials;
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
    /// Groupe 0 : uniformes globaux et matrices des os.
    pub bind_layout: wgpu::BindGroupLayout,
    /// Groupe 1 : matériau du maillage.
    pub material_layout: wgpu::BindGroupLayout,
    /// Maillage skinné, opaque (corps et objets déformés).
    pub mesh: wgpu::RenderPipeline,
    /// Objet rigide : maillage placé par sa matrice, sans poids.
    pub object: wgpu::RenderPipeline,
    /// Os, fantômes et poignées (instanciés, transparence possible).
    pub bone: wgpu::RenderPipeline,
    /// Lignes testées en profondeur (grille).
    pub line: wgpu::RenderPipeline,
    /// Lignes toujours visibles (trajectoires, repères).
    pub overlay_line: wgpu::RenderPipeline,
    /// Arêtes du maillage (fil de fer) : seulement si le GPU sait tracer
    /// les triangles en lignes (`POLYGON_MODE_LINE`).
    pub wire: Option<wgpu::RenderPipeline>,
}

/// Paramètres variables d'un pipeline.
struct Spec<'a> {
    label: &'a str,
    layout: &'a wgpu::PipelineLayout,
    vs: &'a str,
    fs: &'a str,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    topology: wgpu::PrimitiveTopology,
    cull: Option<wgpu::Face>,
    blend: Option<wgpu::BlendState>,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    /// Triangles tracés en lignes, rapprochés de la caméra (fil de fer).
    wire: bool,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bones"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_layout = bindings::globals_layout(device);
        let material_layout = materials::layout(device);
        // Seul le maillage utilise le groupe 1 (matériau) : les autres
        // pipelines n'exigent ainsi que le groupe 0.
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bones"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let mesh_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("maillage"),
            bind_group_layouts: &[Some(&bind_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let create = |s: Spec| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(s.label),
                layout: Some(s.layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(s.vs),
                    compilation_options: Default::default(),
                    buffers: s.buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: s.topology,
                    cull_mode: s.cull,
                    polygon_mode: if s.wire {
                        wgpu::PolygonMode::Line
                    } else {
                        wgpu::PolygonMode::Fill
                    },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(s.depth_write),
                    depth_compare: Some(s.depth_compare),
                    stencil: wgpu::StencilState::default(),
                    bias: if s.wire {
                        wgpu::DepthBiasState {
                            constant: -4,
                            slope_scale: -1.0,
                            clamp: 0.0,
                        }
                    } else {
                        wgpu::DepthBiasState::default()
                    },
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
            layout: &layout,
            vs: "vs_line",
            fs: "fs_line",
            buffers: &lines,
            topology: wgpu::PrimitiveTopology::LineList,
            cull: None,
            blend,
            depth_write: false,
            depth_compare,
            wire: false,
        };

        Self {
            mesh: create(Spec {
                label: "mesh",
                layout: &mesh_layout,
                vs: "vs_mesh",
                fs: "fs_mesh",
                buffers: &mesh_buffers(),
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull: None,
                blend: None,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
                wire: false,
            }),
            object: create(Spec {
                label: "object",
                layout: &mesh_layout,
                vs: "vs_object",
                fs: "fs_mesh",
                buffers: &object_buffers(),
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull: None,
                blend: None,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
                wire: false,
            }),
            bone: create(Spec {
                label: "bones",
                layout: &layout,
                vs: "vs_bone",
                fs: "fs_bone",
                buffers: &bone_buffers(),
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull: Some(wgpu::Face::Back),
                blend,
                depth_write: true,
                depth_compare: wgpu::CompareFunction::Less,
                wire: false,
            }),
            line: create(line("lines", wgpu::CompareFunction::LessEqual)),
            overlay_line: create(line("overlay-lines", wgpu::CompareFunction::Always)),
            wire: device
                .features()
                .contains(wgpu::Features::POLYGON_MODE_LINE)
                .then(|| {
                    create(Spec {
                        label: "wire",
                        layout: &mesh_layout,
                        vs: "vs_mesh",
                        fs: "fs_wire",
                        buffers: &mesh_buffers(),
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull: None,
                        blend,
                        depth_write: false,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        wire: true,
                    })
                }),
            bind_layout,
            material_layout,
        }
    }
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
        for entry in [
            "vs_mesh",
            "vs_object",
            "fs_mesh",
            "fs_wire",
            "vs_bone",
            "fs_bone",
            "vs_line",
            "fs_line",
        ] {
            assert!(
                module.entry_points.iter().any(|e| e.name == entry),
                "{entry} manquant"
            );
        }
    }
}
