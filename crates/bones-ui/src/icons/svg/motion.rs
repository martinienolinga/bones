//! Manipulation : déplacement, IK, sélection, rotation, FK, épingles, chaînes.
//!
//! Glyphes SVG monochromes (viewBox 32 × 32), dessinés en blanc.

pub const MOVE: &str = r##"<path d="M16 4.5V27.5M4.5 16H27.5" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M12.4 8.1 16 4.5 19.6 8.1M12.4 23.9 16 27.5 19.6 23.9M8.1 12.4 4.5 16 8.1 19.6M23.9 12.4 27.5 16 23.9 19.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const ROTATE: &str = r##"<path d="M17.74 6.15A10 10 0 1 1 6.39 13.23" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/><path d="M7.81 10.26L7.07 17.07L1.66 13.29Z" fill="#fff" stroke="#fff" stroke-width="1.2" stroke-linejoin="round"/><circle cx="16" cy="16" r="2.2" fill="#fff"/>"##;

pub const PIN: &str = r##"<path d="M16 28.6C16 28.6 7.4 19.8 7.4 13.4a8.6 8.6 0 0 1 17.2 0C24.6 19.8 16 28.6 16 28.6Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="16" cy="13.2" r="3" fill="#fff"/>"##;

pub const SELECT: &str = r##"<path d="M9 5V24.2L13.6 19.8 17 27.2 20.4 25.7 17 18.4H23.2Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const RESET: &str = r##"<path d="M14.26 6.15A10 10 0 1 0 25.61 13.23" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/><path d="M24.19 10.26L30.34 13.29L24.93 17.07Z" fill="#fff" stroke="#fff" stroke-width="1.2" stroke-linejoin="round"/><path d="M16 10.8V16.4L19.4 18.4" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const IK: &str = r##"<circle cx="24.4" cy="8" r="4.2" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="24.4" cy="8" r="1.6" fill="#fff"/><path d="M18.8 13 21.2 10.8" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-dasharray=".1 2.6"/><path d="M6.4 26 10.6 15.6 18.6 13.2" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><g fill="#fff"><circle cx="6.4" cy="26" r="2.7"/><circle cx="10.6" cy="15.6" r="2.4"/><circle cx="18.6" cy="13.2" r="2.4"/></g>"##;

pub const FK: &str = r##"<path d="M5.6 26.4 13.2 17.6 25.2 15" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M15.12 10.45A7.4 7.4 0 0 1 20.46 16.16" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round"/><path d="M20.57 18.24L18.71 14.27L23.09 14.65Z" fill="#fff" stroke="#fff" stroke-width="1.2" stroke-linejoin="round"/><g fill="#fff"><circle cx="5.6" cy="26.4" r="2.7"/><circle cx="13.2" cy="17.6" r="2.4"/><circle cx="25.2" cy="15" r="2.4"/></g>"##;

pub const CHAIN: &str = r##"<rect x="4.6" y="13.2" width="15" height="8.6" rx="4.3" transform="rotate(-45 12.1 17.5)" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="12.4" y="10.2" width="15" height="8.6" rx="4.3" transform="rotate(-45 19.9 14.5)" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;
