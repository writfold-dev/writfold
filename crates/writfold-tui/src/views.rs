//! View names and keys shared by the parser, tabs and controls.
use crate::app::View;

impl View {
    pub const ALL: [Self; 8] = [
        Self::Progress,
        Self::Gates,
        Self::Timeline,
        Self::Inbox,
        Self::Guards,
        Self::Stale,
        Self::Brief,
        Self::Resume,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Gates => "gates",
            Self::Timeline => "timeline",
            Self::Inbox => "inbox",
            Self::Guards => "guards",
            Self::Stale => "stale",
            Self::Brief => "brief",
            Self::Resume => "resume",
        }
    }

    pub fn key(self) -> char {
        match self {
            Self::Progress => 'p',
            Self::Gates => 'g',
            Self::Timeline => 't',
            Self::Inbox => 'i',
            Self::Guards => 'u',
            Self::Stale => 's',
            Self::Brief => 'b',
            Self::Resume => 'e',
        }
    }

    pub fn digit(self) -> char {
        match self {
            Self::Progress => '1',
            Self::Gates => '2',
            Self::Timeline => '3',
            Self::Inbox => '4',
            Self::Guards => '5',
            Self::Stale => '6',
            Self::Brief => '7',
            Self::Resume => '8',
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.name() == name)
    }

    pub(crate) fn next(self, back: bool) -> Self {
        let at = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
        let offset = if back { Self::ALL.len() - 1 } else { 1 };
        Self::ALL[(at + offset) % Self::ALL.len()]
    }

    pub(crate) fn cache_key(self) -> Self {
        if self == Self::Inbox {
            Self::Resume
        } else {
            self
        }
    }
}
