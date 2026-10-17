//! Encodages binaires : indices et transformations.

use super::super::codec::{decode_indices, encode_indices, pack, unpack};
use crate::Transform;

#[test]
fn indices_round_trip_across_blocks_and_extremes() {
    let mut indices: Vec<u32> = (0..200_000u32).map(|i| (i * 7919) % 150_001).collect();
    indices.extend([0, u32::MAX, 1, u32::MAX - 1, 0, 0, 5]);
    let bytes = encode_indices(&indices);
    assert_eq!(bytes.len(), indices.len() * 4);
    assert_eq!(decode_indices(&bytes), indices);
    assert!(decode_indices(&encode_indices(&[])).is_empty());
}

#[test]
fn neighbouring_indices_encode_as_small_differences() {
    let indices: Vec<u32> = (0..3000).collect();
    let bytes = encode_indices(&indices);
    // Plans des octets de poids fort : uniquement des zéros.
    assert!(bytes[3000..].iter().all(|b| *b == 0));
}

#[test]
fn transforms_pack_exactly_and_reject_non_finite() {
    let t = Transform {
        translation: glam::Vec3::new(1.5, -2.0, 0.25),
        rotation: glam::Quat::from_rotation_y(0.7),
        scale: glam::Vec3::new(1.0, 2.0, 3.0),
    };
    assert_eq!(unpack(&pack(&t)), t);
    let mut bad = pack(&t);
    bad[4] = f32::NAN;
    assert_eq!(unpack(&bad), Transform::IDENTITY);
}
