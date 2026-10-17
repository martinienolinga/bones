//! Cinématique inverse : FABRIK pour les chaînes quelconques, solveur
//! analytique à deux os pour les membres, et points épinglés.

mod fabrik;
mod pins;
#[cfg(test)]
mod tests;
mod two_bone;

pub use fabrik::{IkSettings, solve_fabrik};
pub use pins::{Pin, solve_pins};
pub use two_bone::solve_two_bone;
