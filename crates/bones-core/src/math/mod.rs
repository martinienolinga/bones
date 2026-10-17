//! Mathématiques : transformations, caméra orbitale et sélection à l'écran.

pub mod camera;
pub mod pick;
pub mod transform;

pub use camera::{OrbitCamera, Ray, Viewport};
pub use transform::Transform;

/// Rotation la plus courte qui amène la direction `from` sur la direction `to`.
/// Les deux vecteurs n'ont pas besoin d'être normalisés.
pub(crate) fn rotation_arc(from: glam::Vec3, to: glam::Vec3) -> glam::Quat {
    let (Some(a), Some(b)) = (from.try_normalize(), to.try_normalize()) else {
        return glam::Quat::IDENTITY;
    };
    glam::Quat::from_rotation_arc(a, b)
}
