//! The theme registry; palette values live in `palettes`. The role mapping
//! and the tests are Writfold's.
use crate::palettes::{CARBIDE, CLAY, CLAY_LIGHT, HONE, PAPER, PAPER_DARK, SLATE, VELLUM};

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
    Clay,
    ClayLight,
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
    pub const ALL: [Self; 8] = [
        Self::Clay,
        Self::ClayLight,
        Self::Hone,
        Self::Slate,
        Self::Carbide,
        Self::Vellum,
        Self::Paper,
        Self::PaperDark,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Clay => "clay",
            Self::ClayLight => "clay-light",
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
            Self::Clay => "Neutral charcoal with a terracotta accent; the default",
            Self::ClayLight => "Warm ivory with charcoal ink and a terracotta accent",
            Self::Hone => "Graphite dark with the amber accent",
            Self::Slate => "Low-chroma dark: syntax by luminance, one brass accent",
            Self::Carbide => "High-contrast dark for projectors and bright rooms",
            Self::Vellum => "Warm paper with graphite ink and a burnt-umber accent",
            Self::Paper => "Warm off-white paper with deep green ink and forest accents",
            Self::PaperDark => "Warm green-black paper with soft ivory and sage accents",
        }
    }

    pub const fn is_light(self) -> bool {
        matches!(self, Self::ClayLight | Self::Vellum | Self::Paper)
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        Self::ALL
            .into_iter()
            .find(|id| id.name().eq_ignore_ascii_case(name))
    }

    pub const fn choices() -> &'static str {
        "clay, clay-light, hone, slate, carbide, vellum, paper, paper-dark"
    }

    pub const fn palette(self) -> &'static Palette {
        match self {
            Self::Clay => &CLAY,
            Self::ClayLight => &CLAY_LIGHT,
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
            Self::Clay => Self::ClayLight,
            Self::ClayLight => Self::Hone,
            Self::Hone => Self::Slate,
            Self::Slate => Self::Carbide,
            Self::Carbide => Self::Vellum,
            Self::Vellum => Self::Paper,
            Self::Paper => Self::PaperDark,
            Self::PaperDark => Self::Clay,
        }
    }
}
