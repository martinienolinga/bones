//! Encodages binaires compacts : indices de triangles (différences puis
//! plans d'octets, très bien compressés par deflate) et transformations des
//! poses-clés (10 flottants).

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

/// Indices → différences successives (zigzag) rangées par plans d'octets :
/// des indices voisins donnent de longues suites de zéros.
pub(super) fn encode_indices(indices: &[u32]) -> Vec<u8> {
    let n = indices.len();
    let mut out = vec![0u8; n * 4];
    let mut prev = 0u32;
    for (k, &i) in indices.iter().enumerate() {
        let d = i.wrapping_sub(prev) as i32;
        prev = i;
        let z = ((d << 1) ^ (d >> 31)) as u32;
        for (b, byte) in z.to_le_bytes().into_iter().enumerate() {
            out[b * n + k] = byte;
        }
    }
    out
}

/// Inverse de [`encode_indices`] ; `bytes` contient 4 octets par indice.
pub(super) fn decode_indices(bytes: &[u8]) -> Vec<u32> {
    let n = bytes.len() / 4;
    let (p0, rest) = bytes.split_at(n);
    let (p1, rest) = rest.split_at(n);
    let (p2, p3) = rest.split_at(n);
    let mut prev = 0u32;
    (0..n)
        .map(|k| {
            let z = u32::from_le_bytes([p0[k], p1[k], p2[k], p3[k]]);
            let d = ((z >> 1) as i32) ^ -((z & 1) as i32);
            prev = prev.wrapping_add(d as u32);
            prev
        })
        .collect()
}
