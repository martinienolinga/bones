//! Tampons GPU : tampons dynamiques réutilisés et données statiques du maillage.

use bones_core::{Mesh, Skin};

/// Tampon GPU qui grandit par puissances de deux et n'est jamais réduit :
/// aucune allocation par image une fois la taille de croisière atteinte.
pub(crate) struct GrowBuffer {
    pub buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl GrowBuffer {
    pub fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages, size: u64) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size.max(256),
                usage,
                mapped_at_creation: false,
            }),
            usage,
            label,
        }
    }

    /// Écrit `data` ; retourne `true` si le tampon a été recréé (les groupes
    /// de liaison qui le référencent doivent alors l'être aussi).
    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) -> bool {
        let mut grown = false;
        if data.len() as u64 > self.buffer.size() {
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: (data.len() as u64).next_power_of_two(),
                usage: self.usage,
                mapped_at_creation: false,
            });
            grown = true;
        }
        if !data.is_empty() {
            queue.write_buffer(&self.buffer, 0, data);
        }
        grown
    }
}

/// Maillage résident sur le GPU. Positions, normales, coordonnées de texture
/// et poids sont des tampons séparés, copiés tels quels depuis les tableaux
/// du cœur.
pub(crate) struct MeshBuffers {
    pub positions: wgpu::Buffer,
    pub normals: wgpu::Buffer,
    /// `None` sans coordonnées de texture : on lie alors les positions à leur
    /// place (12 octets par sommet ≥ 8), valeurs ignorées par le shader car
    /// aucune texture n'est appliquée — aucun tampon de zéros à allouer.
    pub uvs: Option<wgpu::Buffer>,
    pub indices: wgpu::Buffer,
    pub weights: Option<wgpu::Buffer>,
}

impl MeshBuffers {
    /// Tampon lié à l'emplacement des coordonnées de texture.
    pub fn uv_source(&self) -> &wgpu::Buffer {
        self.uvs.as_ref().unwrap_or(&self.positions)
    }
}

impl MeshBuffers {
    /// `None` pour un maillage vide.
    pub fn upload(device: &wgpu::Device, queue: &wgpu::Queue, mesh: &Mesh) -> Option<Self> {
        if mesh.indices.is_empty() {
            return None;
        }
        let buffer = |label, data: &[u8], usage| {
            let b = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (data.len() as u64).max(4).next_multiple_of(4),
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&b, 0, data);
            b
        };
        Some(Self {
            positions: buffer(
                "positions",
                bytemuck::cast_slice(&mesh.positions),
                wgpu::BufferUsages::VERTEX,
            ),
            normals: buffer(
                "normals",
                bytemuck::cast_slice(&mesh.normals),
                wgpu::BufferUsages::VERTEX,
            ),
            indices: buffer(
                "indices",
                bytemuck::cast_slice(&mesh.indices),
                wgpu::BufferUsages::INDEX,
            ),
            uvs: mesh
                .has_uvs()
                .then(|| buffer("uvs", bytemuck::cast_slice(&mesh.uvs), wgpu::BufferUsages::VERTEX)),
            weights: None,
        })
    }

    /// Envoie les poids, en réutilisant le tampon existant s'il est assez grand.
    pub fn upload_weights(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, skin: &Skin) {
        let data: &[u8] = bytemuck::cast_slice(&skin.weights);
        let reuse = self
            .weights
            .as_ref()
            .is_some_and(|w| w.size() >= data.len() as u64);
        if !reuse {
            self.weights = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("weights"),
                size: (data.len() as u64).max(16),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let (Some(buffer), false) = (&self.weights, data.is_empty()) {
            queue.write_buffer(buffer, 0, data);
        }
    }
}
