//! Manipulation : déplacement, IK, sélection, rotation, FK, épingles, chaînes.
//!
//! Fragments SVG plats (viewBox 32 × 32) ; `@p`, `@s`, `@d` sont les
//! teintes de la famille (voir `icons::hue`).

pub const MOVE: &str = r##"<path d="M16 3.4 20.6 8.4H17.6V14.4H23.6V11.4L28.6 16 23.6 20.6V17.6H17.6V23.6H20.6L16 28.6 11.4 23.6H14.4V17.6H8.4V20.6L3.4 16 8.4 11.4V14.4H14.4V8.4H11.4Z" fill="@p" stroke="@p" stroke-width="1.2" stroke-linejoin="round"/><circle cx="16" cy="16" r="2.6" fill="#fff"/>"##;

pub const ROTATE: &str = r##"<path d="M17.74 6.15A10 10 0 1 1 6.33 13.44" fill="none" stroke="@p" stroke-width="2.8" stroke-linecap="round"/><path d="M7.81 10.26L7.09 17.57L1.19 13.44Z" fill="@p" stroke="@p" stroke-width="1.2" stroke-linejoin="round"/><circle cx="16" cy="16" r="3.3" fill="@s"/>"##;

pub const PIN: &str = r##"<path d="M16 29.2C16 29.2 6.8 19.6 6.8 12.9a9.2 9.2 0 0 1 18.4 0C25.2 19.6 16 29.2 16 29.2Z" fill="@p"/><circle cx="16" cy="12.8" r="3.8" fill="#fff"/>"##;

pub const SELECT: &str = r##"<path d="M8 4.5V24.8L12.9 20.2 16.4 27.8 20.2 26.1 16.8 18.6H23.6Z" fill="@p" stroke="@p" stroke-width="1.6" stroke-linejoin="round"/>"##;

pub const RESET: &str = r##"<path d="M14.26 6.15A10 10 0 1 0 25.67 13.44" fill="none" stroke="@p" stroke-width="2.8" stroke-linecap="round"/><path d="M24.19 10.26L30.81 13.44L24.91 17.57Z" fill="@p" stroke="@p" stroke-width="1.2" stroke-linejoin="round"/><path d="M16 10.5V16.4L19.6 18.6" stroke="@s" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const IK: &str = r##"<circle cx="24.6" cy="8" r="4.8" stroke="@s" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="24.6" cy="8" r="1.8" fill="@s"/><path d="M18.6 13.2 21.6 10.6" stroke="#fff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-opacity=".7" stroke-dasharray="1.6 1.8"/><path d="M6.4 26 10.6 15.6 18.6 13.2" stroke="@p" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="6.4" cy="26" r="2.8" fill="@p"/><circle cx="10.6" cy="15.6" r="2.3" fill="#fff"/><circle cx="18.6" cy="13.2" r="2.3" fill="#fff"/>"##;

pub const FK: &str = r##"<path d="M5.6 26.4 13.2 17.6 25.2 15" stroke="@p" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M15.12 10.45A7.4 7.4 0 0 1 20.46 16.16" fill="none" stroke="@s" stroke-width="2" stroke-linecap="round"/><path d="M20.57 18.24L18.61 14.26L23.19 14.66Z" fill="@s" stroke="@s" stroke-width="1.2" stroke-linejoin="round"/><circle cx="5.6" cy="26.4" r="2.8" fill="@p"/><circle cx="13.2" cy="17.6" r="2.6" fill="#fff"/><circle cx="25.2" cy="15" r="2.2" fill="@p"/>"##;

pub const CHECK: &str = r##"<path d="M6.6 16.8 13 23.2 25.4 9.4" stroke="@p" stroke-width="3.6" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const CHAIN: &str = r##"<rect x="4.6" y="13.2" width="15" height="8.6" rx="4.3" transform="rotate(-45 12.1 17.5)" stroke="@p" fill="none" stroke-width="3.2"/><rect x="12.4" y="10.2" width="15" height="8.6" rx="4.3" transform="rotate(-45 19.9 14.5)" stroke="@s" fill="none" stroke-width="3.2"/>"##;
