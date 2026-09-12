//! The ui/ HTML references Inter (400/500/600/700) and JetBrains Mono via
//! Google Fonts. The app bundles the same faces (latin subset, OFL) so the
//! native chrome renders with identical metrics — advance widths, kerning,
//! ascender/descender — instead of falling back to whatever the system has.
//! Source: fontsource CDN (jsDelivr), files committed under assets/fonts/.

pub const INTER_400: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Inter-400.ttf"
));
pub const INTER_500: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Inter-500.ttf"
));
pub const INTER_600: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Inter-600.ttf"
));
pub const INTER_700: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Inter-700.ttf"
));
pub const JBM_400: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/JBM-400.ttf"
));
