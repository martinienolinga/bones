//! Manipulation : déplacement, IK, sélection, rotation, FK, épingles, chaînes.
//!
//! Fragments SVG (viewBox 32 × 32) ; voir `icons::hue` pour les dégradés.

pub const MOVE: &str = r##"<path d="M16 3.4 20.6 8.4H17.6V14.4H23.6V11.4L28.6 16 23.6 20.6V17.6H17.6V23.6H20.6L16 28.6 11.4 23.6H14.4V17.6H8.4V20.6L3.4 16 8.4 11.4V14.4H14.4V8.4H11.4Z" fill="url(#a)" stroke="url(#a)" stroke-width="1.2" stroke-linejoin="round"/><circle cx="16" cy="16" r="2.6" fill="#fff"/><path d="M14.6 6.6 16 5" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.1"/>"##;

pub const ROTATE: &str = r##"<path d="M17.74 6.15A10 10 0 1 1 6.33 13.44" fill="none" stroke="url(#a)" stroke-width="3.2" stroke-linecap="round"/><path d="M7.81 10.26L7.09 17.57L1.19 13.44Z" fill="url(#a)" stroke="url(#a)" stroke-width="1.2" stroke-linejoin="round"/><circle cx="16" cy="16" r="3.3" fill="url(#b)"/><circle cx="15" cy="15" r="1.1" fill="#fff" fill-opacity=".85"/>"##;

pub const PIN: &str = r##"<path d="M16 29.2C16 29.2 6.8 19.6 6.8 12.9a9.2 9.2 0 0 1 18.4 0C25.2 19.6 16 29.2 16 29.2Z" fill="url(#a)"/><circle cx="16" cy="12.8" r="3.8" fill="#fff"/><path d="M10.2 9.4a6.6 6.6 0 0 1 4-3.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.4"/>"##;

pub const SELECT: &str = r##"<path d="M8 4.5V24.8L12.9 20.2 16.4 27.8 20.2 26.1 16.8 18.6H23.6Z" fill="url(#a)" stroke="#fff" stroke-opacity=".9" stroke-width="1.4" stroke-linejoin="round"/><path d="M10 9V18.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.2"/>"##;

pub const RESET: &str = r##"<path d="M14.26 6.15A10 10 0 1 0 25.67 13.44" fill="none" stroke="url(#a)" stroke-width="3.2" stroke-linecap="round"/><path d="M24.19 10.26L30.81 13.44L24.91 17.57Z" fill="url(#a)" stroke="url(#a)" stroke-width="1.2" stroke-linejoin="round"/><path d="M16 10.5V16.4L19.6 18.6" stroke="url(#b)" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const IK: &str = r##"<circle cx="24.6" cy="8" r="4.8" fill="url(#b)" fill-opacity=".25" stroke="url(#b)" stroke-width="1.8"/><circle cx="24.6" cy="8" r="1.8" fill="#fff"/><path d="M18.6 13.2 21.6 10.6" stroke="#fff" stroke-opacity=".7" stroke-width="1.5" stroke-dasharray="1.6 1.8" stroke-linecap="round"/><path d="M6.4 26 10.6 15.6 18.6 13.2" stroke="url(#a)" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="6.4" cy="26" r="2.8" fill="url(#a)"/><circle cx="10.6" cy="15.6" r="2.3" fill="#fff"/><circle cx="18.6" cy="13.2" r="2.3" fill="#fff"/>"##;

pub const FK: &str = r##"<path d="M5.6 26.4 13.2 17.6 25.2 15" stroke="url(#a)" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M15.12 10.45A7.4 7.4 0 0 1 20.46 16.16" fill="none" stroke="url(#b)" stroke-width="2" stroke-linecap="round"/><path d="M20.57 18.24L18.61 14.26L23.19 14.66Z" fill="url(#b)" stroke="url(#b)" stroke-width="1.2" stroke-linejoin="round"/><circle cx="5.6" cy="26.4" r="2.8" fill="url(#a)"/><circle cx="13.2" cy="17.6" r="2.6" fill="#fff"/><circle cx="25.2" cy="15" r="2.2" fill="url(#a)"/>"##;

pub const CHECK: &str = r##"<path d="M6.6 16.8 13 23.2 25.4 9.4" stroke="url(#a)" stroke-width="3.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const CHAIN: &str = r##"<rect x="4.6" y="13.2" width="15" height="8.6" rx="4.3" transform="rotate(-45 12.1 17.5)" stroke="url(#a)" fill="none" stroke-width="3.3"/><rect x="12.4" y="10.2" width="15" height="8.6" rx="4.3" transform="rotate(-45 19.9 14.5)" stroke="url(#b)" fill="none" stroke-width="3.3"/>"##;
