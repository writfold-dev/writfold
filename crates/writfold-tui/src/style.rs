//! Pure colour selection and Writfold's semantic roles.
use crate::lines::Tone;
use crate::theme::{Rgb, ThemeId};
use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub colorterm: Option<String>,
    pub no_color: bool,
    pub term: Option<String>,
    pub theme: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Depth {
    Truecolor,
    Ansi,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Look {
    pub theme: ThemeId,
    pub depth: Depth,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            theme: ThemeId::default(),
            depth: Depth::Ansi,
        }
    }
}

impl Look {
    pub fn label(self) -> &'static str {
        match self.depth {
            Depth::Truecolor => self.theme.name(),
            Depth::Ansi => "ansi",
            Depth::None => "none",
        }
    }
}

pub fn depth(flag: Option<&str>, env: &Environment) -> Result<Depth, String> {
    match flag {
        Some("truecolor") => return Ok(Depth::Truecolor),
        Some("ansi") => return Ok(Depth::Ansi),
        Some("none") => return Ok(Depth::None),
        None | Some("auto") => {}
        Some(other) => {
            return Err(format!(
                "--color must be auto, truecolor, ansi or none, not {other:?}"
            ));
        }
    }
    if env.no_color || env.term.as_deref() == Some("dumb") {
        Ok(Depth::None)
    } else if matches!(env.colorterm.as_deref(), Some("truecolor" | "24bit")) {
        Ok(Depth::Truecolor)
    } else {
        Ok(Depth::Ansi)
    }
}

pub fn choose_theme(
    flag: Option<&str>,
    env: Option<&str>,
    config: Option<&str>,
) -> (ThemeId, Option<String>) {
    let selected = flag
        .map(|n| (n, "--theme"))
        .or_else(|| env.map(|n| (n, "WRITFOLD_THEME")))
        .or_else(|| config.map(|n| (n, "config.toml")));
    let Some((name, source)) = selected else {
        return (ThemeId::default(), None);
    };
    match ThemeId::from_name(name) {
        Some(theme) => (theme, None),
        None => (
            ThemeId::default(),
            Some(format!(
                "watch: unknown theme {name:?} ({source}); using hone; themes: {}",
                ThemeId::choices()
            )),
        ),
    }
}

fn rgb(value: Rgb) -> Color {
    Color::Rgb(value.0, value.1, value.2)
}

fn ansi(tone: Tone) -> Style {
    match tone {
        Tone::Plain | Tone::Done | Tone::Head | Tone::Key => Style::default(),
        Tone::Dim | Tone::Skipped | Tone::Later => Style::default().fg(Color::DarkGray),
        Tone::Good => Style::default().fg(Color::Green),
        Tone::Vouched => Style::default().fg(Color::Magenta),
        Tone::Info => Style::default().fg(Color::Blue),
        Tone::Warn => Style::default().fg(Color::Yellow),
        Tone::Current => Style::default().fg(Color::Cyan),
        Tone::Bad => Style::default().fg(Color::Red),
    }
}

pub fn style(look: Look, tone: Tone) -> Style {
    let palette = look.theme.palette();
    let mut style = match look.depth {
        Depth::None => Style::default(),
        Depth::Ansi => ansi(tone),
        Depth::Truecolor => {
            let ink = match tone {
                Tone::Plain | Tone::Head => palette.text,
                Tone::Done => palette.secondary,
                Tone::Dim | Tone::Skipped | Tone::Later => palette.muted,
                Tone::Good => palette.good,
                Tone::Vouched => palette.vouched,
                Tone::Info => palette.info,
                Tone::Warn => palette.warning,
                Tone::Bad => palette.error,
                Tone::Current | Tone::Key => palette.accent,
            };
            Style::default().fg(rgb(ink))
        }
    };
    if matches!(tone, Tone::Bad | Tone::Head | Tone::Current | Tone::Key) {
        style = style.add_modifier(Modifier::BOLD);
    }
    if look.depth == Depth::Truecolor && tone == Tone::Key {
        style = style.bg(rgb(palette.raised));
    }
    style
}

fn surface(look: Look, background: Rgb) -> Style {
    let style = style(look, Tone::Plain);
    if look.depth == Depth::Truecolor {
        style.bg(rgb(background))
    } else {
        style
    }
}

pub fn base(look: Look) -> Style {
    surface(look, look.theme.palette().base)
}
pub fn bar(look: Look) -> Style {
    surface(look, look.theme.palette().surface)
}
pub fn raised(look: Look) -> Style {
    surface(look, look.theme.palette().raised)
}

pub fn selected(look: Look) -> Style {
    if look.depth == Depth::Truecolor {
        surface(look, look.theme.palette().selection)
    } else {
        Style::default()
            .fg(Color::Reset)
            .add_modifier(Modifier::REVERSED)
    }
}

pub fn border(look: Look, focused: bool) -> Style {
    match look.depth {
        Depth::Truecolor => base(look).fg(rgb(if focused {
            look.theme.palette().accent
        } else {
            look.theme.palette().faint
        })),
        Depth::Ansi => Style::default().fg(if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }),
        Depth::None => Style::default(),
    }
}
