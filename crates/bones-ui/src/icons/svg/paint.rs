//! Peinture de poids : pinceau, flou, moyenne, étalement, dégradé, pipette,
//! normalisation.
//!
//! Glyphes SVG monochromes (viewBox 32 × 32), dessinés en blanc, dans le même
//! style que les autres familles (traits de 2,4, extrémités rondes, aplats).

pub const BRUSH: &str = r##"<path d="M24.8 7.2 16.6 15.4" stroke="#fff" stroke-width="2.8" stroke-linecap="round"/><path d="M16.8 12.8 19.2 15.2 13.6 23.4C12.3 25.1 8.2 25.6 6.7 25.3 6.4 23.8 6.9 19.7 8.6 18.4Z" fill="#fff" stroke="#fff" stroke-width="1" stroke-linejoin="round"/>"##;

pub const BLUR: &str = r##"<circle cx="16" cy="16" r="4.6" fill="#fff"/><circle cx="16" cy="16" r="10.6" stroke="#fff" stroke-width="2.2" stroke-linecap="round" fill="none" stroke-dasharray=".1 4.2"/>"##;

pub const AVERAGE: &str = r##"<circle cx="16" cy="16" r="9.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" fill="none"/><path d="M8.4 16H23.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/>"##;

pub const SMEAR: &str = r##"<path d="M5.6 11.2C11 9.4 20 13.4 26.4 10.4" stroke="#fff" stroke-width="2.8" stroke-linecap="round" fill="none"/><path d="M5.6 17C11 15.2 20 19.2 26.4 16.2" stroke="#fff" stroke-width="2.2" stroke-linecap="round" fill="none"/><path d="M5.6 22.8C11 21 20 25 26.4 22" stroke="#fff" stroke-width="1.6" stroke-linecap="round" fill="none"/>"##;

pub const GRADIENT: &str = r##"<path d="M6.6 9.4H25.4M6.6 16H20.6M6.6 22.6H15.2" stroke="#fff" stroke-width="3.4" stroke-linecap="round"/>"##;

pub const SAMPLE: &str = r##"<circle cx="22.6" cy="9.4" r="3.8" stroke="#fff" stroke-width="2.2" fill="none"/><path d="M20 6.8 25.2 12" stroke="#fff" stroke-width="2.2" stroke-linecap="round"/><path d="M20.4 11.6 8 24 6 26M9.6 22.4 12 24.8" stroke="#fff" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const NORMALIZE: &str = r##"<path d="M4.4 7.6H27.6" stroke="#fff" stroke-width="2.2" stroke-linecap="round"/><path d="M7 26.4V14.4M13 26.4V10.4M19 26.4V16.4M25 26.4V12.4" stroke="#fff" stroke-width="3.2" stroke-linecap="round"/>"##;
