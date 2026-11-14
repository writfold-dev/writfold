//! Palettes ported by value from strop (MIT, same author),
//! `crates/strop-core/src/theme/`; the role mapping and the tests are Writfold's.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Palette {
    pub base: Rgb,
    pub surface: Rgb,
    pub raised: Rgb,
    pub selection: Rgb,
    pub text: Rgb,
    pub secondary: Rgb,
    pub muted: Rgb,
    pub faint: Rgb,
    pub on_accent: Rgb,
    pub accent: Rgb,
    pub error: Rgb,
    pub warning: Rgb,
    pub info: Rgb,
    pub good: Rgb,
    pub vouched: Rgb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeId {
    Hone,
    Slate,
    Carbide,
    Vellum,
    Paper,
    PaperDark,
}

impl Default for ThemeId {
    fn default() -> Self {
        Self::ALL[0]
    }
}

impl ThemeId {
    pub const ALL: [Self; 6] = [
        Self::Hone,
        Self::Slate,
        Self::Carbide,
        Self::Vellum,
        Self::Paper,
        Self::PaperDark,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Hone => "hone",
            Self::Slate => "slate",
            Self::Carbide => "carbide",
            Self::Vellum => "vellum",
            Self::Paper => "paper",
            Self::PaperDark => "paper-dark",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Hone => "Graphite dark with the amber accent; the default",
            Self::Slate => "Low-chroma dark: syntax by luminance, one brass accent",
            Self::Carbide => "High-contrast dark for projectors and bright rooms",
            Self::Vellum => "Warm paper with graphite ink and a burnt-umber accent",
            Self::Paper => "Warm off-white paper with deep green ink and forest accents",
            Self::PaperDark => "Warm green-black paper with soft ivory and sage accents",
        }
    }

    pub const fn is_light(self) -> bool {
        matches!(self, Self::Vellum | Self::Paper)
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        Self::ALL
            .into_iter()
            .find(|id| id.name().eq_ignore_ascii_case(name))
    }

    pub const fn choices() -> &'static str {
        "hone, slate, carbide, vellum, paper, paper-dark"
    }

    pub const fn palette(self) -> &'static Palette {
        match self {
            Self::Hone => &HONE,
            Self::Slate => &SLATE,
            Self::Carbide => &CARBIDE,
            Self::Vellum => &VELLUM,
            Self::Paper => &PAPER,
            Self::PaperDark => &PAPER_DARK,
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Hone => Self::Slate,
            Self::Slate => Self::Carbide,
            Self::Carbide => Self::Vellum,
            Self::Vellum => Self::Paper,
            Self::Paper => Self::PaperDark,
            Self::PaperDark => Self::Hone,
        }
    }
}

const HONE: Palette = Palette {
    base: Rgb(0x15, 0x17, 0x1c),
    surface: Rgb(0x1c, 0x1f, 0x25),
    raised: Rgb(0x23, 0x27, 0x2f),
    selection: Rgb(0x30, 0x40, 0x5a),
    text: Rgb(0xe6, 0xe1, 0xd6),
    secondary: Rgb(0xa8, 0xad, 0xb8),
    muted: Rgb(0x85, 0x8c, 0x9a),
    faint: Rgb(0x4a, 0x51, 0x60),
    on_accent: Rgb(0x15, 0x17, 0x1c),
    accent: Rgb(0xe8, 0xa8, 0x48),
    error: Rgb(0xe8, 0x70, 0x7c),
    warning: Rgb(0xd9, 0xa8, 0x5a),
    info: Rgb(0x7f, 0xb2, 0xd4),
    good: Rgb(0x8f, 0xc4, 0x8a),
    vouched: Rgb(0xb7, 0x9c, 0xe6),
};

const SLATE: Palette = Palette {
    base: Rgb(0x1a, 0x1c, 0x20),
    surface: Rgb(0x20, 0x23, 0x29),
    raised: Rgb(0x27, 0x2b, 0x32),
    selection: Rgb(0x34, 0x3a, 0x45),
    text: Rgb(0xdc, 0xd9, 0xd2),
    secondary: Rgb(0xae, 0xb1, 0xb7),
    muted: Rgb(0x85, 0x8a, 0x93),
    faint: Rgb(0x54, 0x59, 0x62),
    on_accent: Rgb(0x1a, 0x1c, 0x20),
    accent: Rgb(0xc9, 0xb2, 0x7e),
    error: Rgb(0xd9, 0x8a, 0x90),
    warning: Rgb(0xcf, 0xb2, 0x7a),
    info: Rgb(0x92, 0xb2, 0xc8),
    good: Rgb(0xa5, 0xbd, 0x9a),
    vouched: Rgb(0xb8, 0xaa, 0xcb),
};

const CARBIDE: Palette = Palette {
    base: Rgb(0x0b, 0x0c, 0x0f),
    surface: Rgb(0x14, 0x16, 0x1b),
    raised: Rgb(0x1d, 0x20, 0x27),
    selection: Rgb(0x2f, 0x4a, 0x6e),
    text: Rgb(0xf4, 0xf2, 0xec),
    secondary: Rgb(0xcf, 0xd3, 0xda),
    muted: Rgb(0xa3, 0xa9, 0xb4),
    faint: Rgb(0x6a, 0x72, 0x80),
    on_accent: Rgb(0x0b, 0x0c, 0x0f),
    accent: Rgb(0xff, 0xb7, 0x4a),
    error: Rgb(0xff, 0x7b, 0x86),
    warning: Rgb(0xf0, 0xc0, 0x60),
    info: Rgb(0x8f, 0xc8, 0xff),
    good: Rgb(0x9b, 0xe0, 0x8f),
    vouched: Rgb(0xcf, 0xaa, 0xff),
};

const VELLUM: Palette = Palette {
    base: Rgb(0xf6, 0xf2, 0xe9),
    surface: Rgb(0xfd, 0xfb, 0xf6),
    raised: Rgb(0xe5, 0xdf, 0xd0),
    selection: Rgb(0xd1, 0xc9, 0xb3),
    text: Rgb(0x26, 0x2a, 0x30),
    secondary: Rgb(0x4b, 0x52, 0x5c),
    muted: Rgb(0x65, 0x6d, 0x7b),
    faint: Rgb(0xae, 0xb3, 0xba),
    on_accent: Rgb(0xf6, 0xf2, 0xe9),
    accent: Rgb(0x8a, 0x51, 0x0f),
    error: Rgb(0xb8, 0x32, 0x3f),
    warning: Rgb(0x8a, 0x5d, 0x13),
    info: Rgb(0x2a, 0x6a, 0x94),
    good: Rgb(0x3f, 0x7a, 0x3a),
    vouched: Rgb(0x6a, 0x3f, 0xaa),
};

const PAPER: Palette = Palette {
    base: Rgb(0xf8, 0xf8, 0xf3),
    surface: Rgb(0xee, 0xf1, 0xe8),
    raised: Rgb(0xdb, 0xe2, 0xd6),
    selection: Rgb(0xc2, 0xd2, 0xba),
    text: Rgb(0x20, 0x36, 0x2d),
    secondary: Rgb(0x3a, 0x4d, 0x43),
    muted: Rgb(0x47, 0x59, 0x51),
    faint: Rgb(0xa9, 0xb8, 0xa8),
    on_accent: Rgb(0xf8, 0xf8, 0xf3),
    accent: Rgb(0x1a, 0x66, 0x50),
    error: Rgb(0x9c, 0x3f, 0x38),
    warning: Rgb(0x7c, 0x5a, 0x20),
    info: Rgb(0x25, 0x67, 0x7a),
    good: Rgb(0x3b, 0x7a, 0x4a),
    vouched: Rgb(0x6d, 0x5a, 0x8c),
};

const PAPER_DARK: Palette = Palette {
    base: Rgb(0x15, 0x1d, 0x19),
    surface: Rgb(0x1c, 0x27, 0x20),
    raised: Rgb(0x24, 0x32, 0x29),
    selection: Rgb(0x2f, 0x4b, 0x3a),
    text: Rgb(0xe3, 0xe9, 0xdd),
    secondary: Rgb(0xbf, 0xcc, 0xbe),
    muted: Rgb(0x9d, 0xaa, 0x9e),
    faint: Rgb(0x53, 0x69, 0x5b),
    on_accent: Rgb(0x15, 0x1d, 0x19),
    accent: Rgb(0x9b, 0xc7, 0xac),
    error: Rgb(0xdf, 0x99, 0x92),
    warning: Rgb(0xd3, 0xb4, 0x77),
    info: Rgb(0x9a, 0xba, 0xc5),
    good: Rgb(0xa8, 0xbb, 0x87),
    vouched: Rgb(0xb7, 0xa6, 0xc9),
};
