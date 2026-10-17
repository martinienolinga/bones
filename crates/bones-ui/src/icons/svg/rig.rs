//! Os et rigging : os, édition, pose, symétrie, auto-rig, hiérarchie, marqueurs.
//!
//! Fragments SVG plats (viewBox 32 × 32) ; `@p`, `@s`, `@d` sont les
//! teintes de la famille (voir `icons::hue`).

pub const BONE: &str = r##"<g fill="@p"><circle cx="10.4" cy="7.6" r="3.7"/><circle cx="7.6" cy="10.4" r="3.7"/><circle cx="24.4" cy="21.6" r="3.7"/><circle cx="21.6" cy="24.4" r="3.7"/><path d="M10.4 7.6 24.4 21.6 21.6 24.4 7.6 10.4Z"/></g>"##;

pub const BONE_ADD: &str = r##"<g transform="matrix(.8 0 0 .8 .6 5.8)"><g fill="@p"><circle cx="10.4" cy="7.6" r="3.7"/><circle cx="7.6" cy="10.4" r="3.7"/><circle cx="24.4" cy="21.6" r="3.7"/><circle cx="21.6" cy="24.4" r="3.7"/><path d="M10.4 7.6 24.4 21.6 21.6 24.4 7.6 10.4Z"/></g></g><circle cx="23.5" cy="8.5" r="7.6" fill="#1C1C1E"/><circle cx="23.5" cy="8.5" r="6" fill="#30D158"/><path d="M20.5 8.5H26.5M23.5 5.5V11.5" stroke="#fff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const MIRROR: &str = r##"<path d="M16 3.5V28.5" stroke="#fff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-opacity=".55" stroke-dasharray="2.2 2.4"/><path d="M13 7.6V24.6H4.6Z" fill="@p" stroke="@p" stroke-width="1.6" stroke-linejoin="round"/><path d="M19 7.6V24.6H27.4Z" stroke="@p" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const SYMMETRIZE: &str = r##"<path d="M16 9.5V28.5" stroke="#fff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-opacity=".55" stroke-dasharray="2.2 2.4"/><path d="M13 13V26.8H5.6Z" fill="@p" stroke="@p" stroke-width="1.6" stroke-linejoin="round"/><path d="M19 13V26.8H26.4Z" fill="@s" stroke="@s" stroke-width="1.6" stroke-linejoin="round"/><path d="M8.29 10.7A8.2 8.2 0 0 1 22.12 8.05" fill="none" stroke="#fff" stroke-width="2.2" stroke-linecap="round"/><path d="M23.49 10.16L19.24 7.02L24 4.9Z" fill="#fff" stroke="#fff" stroke-width="1.2" stroke-linejoin="round"/>"##;

pub const AUTO_RIG: &str = r##"<path d="M6 26.5 18.4 14.1" stroke="@s" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M15.6 16.9 18.4 14.1" stroke="#fff" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M22.2 3.6Q23.94 8.06 28.4 9.8Q23.94 11.54 22.2 16Q20.46 11.54 16 9.8Q20.46 8.06 22.2 3.6Z" fill="@p"/><path d="M27 16Q27.73 17.87 29.6 18.6Q27.73 19.33 27 21.2Q26.27 19.33 24.4 18.6Q26.27 17.87 27 16Z" fill="#fff"/><path d="M12.6 3.2Q13.27 4.93 15 5.6Q13.27 6.27 12.6 8Q11.93 6.27 10.2 5.6Q11.93 4.93 12.6 3.2Z" fill="@s"/><circle cx="27.2" cy="5.2" r="1.1" fill="#fff"/>"##;

pub const HIERARCHY: &str = r##"<path d="M16 10.5V15.5M8.5 21V15.5H23.5V21" stroke="@s" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="11" y="3.5" width="10" height="7.2" rx="2.4" fill="@p"/><rect x="3.5" y="21" width="10" height="7.2" rx="2.4" fill="@s"/><rect x="18.5" y="21" width="10" height="7.2" rx="2.4" fill="@s"/>"##;

pub const MARKER: &str = r##"<circle cx="16" cy="16" r="10" fill="@p" fill-opacity=".16" stroke="@p" stroke-width="2.2"/><path d="M16 2.8V7.6M16 24.4V29.2M2.8 16H7.6M24.4 16H29.2" stroke="@p" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="16" cy="16" r="4.2" fill="@p"/>"##;

pub const POSE: &str = r##"<circle cx="17" cy="6.2" r="3.4" fill="@p"/><path d="M16.4 11 14.6 18.4" stroke="@p" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M7.8 9.6 12 13.4 16.2 12.2 21.4 9.2 24.6 5.4" stroke="@p" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M9.8 27.4 11.4 22 14.6 18.4 19.6 21.4 21.2 27.2" stroke="@p" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><g fill="#fff"><circle cx="16.2" cy="12.2" r="1.3"/><circle cx="14.6" cy="18.4" r="1.3"/><circle cx="11.4" cy="22" r="1.1"/><circle cx="19.6" cy="21.4" r="1.1"/></g>"##;

pub const EDIT: &str = r##"<path d="M6.4 25.6 8 19.6 20.9 6.7a2.8 2.8 0 0 1 4 0l.4.4a2.8 2.8 0 0 1 0 4L12.4 24Z" fill="@p"/><path d="M6.4 25.6 8 19.6 12.4 24Z" fill="@s"/><path d="M6.4 25.6 7 23.4 8.6 25Z" fill="#1C1C1E"/><path d="M18.4 8.8 23.2 13.6" stroke="@d" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;
