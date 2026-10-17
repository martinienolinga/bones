//! Encodages binaires compacts : indices de triangles (différences puis
//! plans d'octets, très bien compressés par deflate) et transformations des
//! poses-clés (10 flottants).

use rayon::prelude::*;

use crate::Transform;

/// Transformation enregistrée : translation (3), rotation (4), échelle (3).
pub(super) type PackedTransform = [f32; 10];

pub(super) fn pack(t: &Transform) -> PackedTransform {
    let (p, r, s) = (t.translation, t.rotation, t.scale);
    [p.x, p.y, p.z, r.x, r.y, r.z, r.w, s.x, s.y, s.z]
}

/// Transformation lue ; une valeur non finie redonne l'identité.
pub(super) fn unpack(v: &PackedTransform) -> Transform {
    if !v.iter().all(|x| x.is_finite()) {
        return Transform::IDENTITY;
    }
    Transform {
        translation: glam::Vec3::new(v[0], v[1], v[2]),
        rotation: glam::Quat::from_xyzw(v[3], v[4], v[5], v[6]),
        scale: glam::Vec3::new(v[7], v[8], v[9]),
    }
}

/// Taille des blocs traités en parallèle.
const CHUNK: usize = 1 << 16;

/// Indices → différences successives (zigzag) rangées par plans d'octets :
/// des indices voisins donnent de longues suites de zéros. Un plan et un
/// bloc par tâche.
pub(super) fn encode_indices(indices: &[u32]) -> Vec<u8> {
    let n = indices.len();
    let mut out = vec![0u8; n * 4];
    out.par_chunks_mut(n.max(1)).enumerate().for_each(|(b, plane)| {
        plane.par_chunks_mut(CHUNK).enumerate().for_each(|(c, part)| {
            let start = c * CHUNK;
            let mut prev = start.checked_sub(1).map_or(0, |p| indices[p]);
            for (dst, &i) in part.iter_mut().zip(&indices[start..]) {
                *dst = (zigzag(i.wrapping_sub(prev)) >> (8 * b)) as u8;
                prev = i;
            }
        });
    });
    out
}

/// Inverse de [`encode_indices`] ; `bytes` contient 4 octets par indice.
pub(super) fn decode_indices(bytes: &[u8]) -> Vec<u32> {
    let n = bytes.len() / 4;
    let (p0, rest) = bytes.split_at(n);
    let (p1, rest) = rest.split_at(n);
    let (p2, p3) = rest.split_at(n);
    let mut out = vec![0u32; n];
    out.par_chunks_mut(CHUNK).enumerate().for_each(|(c, part)| {
        let start = c * CHUNK;
        for (k, v) in part.iter_mut().enumerate() {
            let j = start + k;
            *v = unzigzag(u32::from_le_bytes([p0[j], p1[j], p2[j], p3[j]]));
        }
    });
    // Somme cumulée des différences (séquentielle, très rapide).
    let mut prev = 0u32;
    for v in &mut out {
        prev = prev.wrapping_add(*v);
        *v = prev;
    }
    out
}

fn zigzag(d: u32) -> u32 {
    let d = d as i32;
    ((d << 1) ^ (d >> 31)) as u32
}

fn unzigzag(z: u32) -> u32 {
    ((z >> 1) as i32 ^ -((z & 1) as i32)) as u32
}
