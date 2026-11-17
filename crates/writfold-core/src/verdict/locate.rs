//! Where the cited span sits in the file at the target commit. The
//! decisions are [`window`] (whether to search, and which window was
//! cited) and [`place`] (from the windows' flags to one answer); both are
//! translated and proved. [`locate`] drives them with the caller's span
//! comparison, which reads text and is tested, not translated.

use super::Located;
use alloc::vec::Vec;

/// What to search for, from the file's length and the cited lines.
pub enum Window {
    /// The file is longer than the search limit.
    Unsearched,
    /// No window of the cited height fits, or the cited lines are not a
    /// range (`first` is 0 or `last < first`).
    Absent,
    /// Windows of `height` lines; `cited` is the cited window's start
    /// (zero-based) when it lies inside the file.
    Search { cited: Option<usize>, height: usize },
}

/// Which windows to look at for cited lines `first..=last` (one-based) in
/// a file of `lines` lines, searched only up to `limit` lines.
pub fn window(lines: usize, first: usize, last: usize, limit: usize) -> Window {
    if lines > limit {
        return Window::Unsearched;
    }
    if first == 0 || last < first {
        return Window::Absent;
    }
    let height = last - first + 1;
    if height > lines {
        return Window::Absent;
    }
    let cited = first - 1;
    if last <= lines {
        Window::Search {
            cited: Some(cited),
            height,
        }
    } else {
        Window::Search {
            cited: None,
            height,
        }
    }
}

/// Where the span is, by window start (zero-based).
pub enum Place {
    /// The cited window carries the span digest.
    Same,
    /// The first window that carries it starts here.
    At(usize),
    /// No window carries it.
    Absent,
}

/// The answer from the flags: `cited`, whether the cited window carries
/// the span digest; `windows[s]`, whether the window starting at `s` does,
/// from the top. The cited window first, then the first window that holds
/// the span.
pub fn place(cited: bool, windows: &[bool]) -> Place {
    if cited {
        return Place::Same;
    }
    let mut start = 0;
    while start < windows.len() {
        if windows[start] {
            return Place::At(start);
        }
        start += 1;
    }
    Place::Absent
}

/// Slide a window of the cited height over a file of `lines` lines: the
/// cited position first, then every position from the top.
/// `matches(start, height)` says whether the zero-based window
/// `start..start + height` carries the cited span digest. The windows
/// from the top are compared only while none has matched, and only when
/// the cited one does not: [`place`] reads the flags up to the first hit.
pub fn locate(
    lines: usize,
    first: u32,
    last: u32,
    limit: usize,
    matches: impl Fn(usize, usize) -> bool,
) -> Located {
    let (Ok(first), Ok(last)) = (usize::try_from(first), usize::try_from(last)) else {
        return Located::Absent;
    };
    let (cited, height) = match window(lines, first, last, limit) {
        Window::Unsearched => return Located::Unsearched,
        Window::Absent => return Located::Absent,
        Window::Search { cited, height } => (cited, height),
    };
    let cited = cited.is_some_and(|start| matches(start, height));
    let mut windows: Vec<bool> = Vec::new();
    let mut start = 0;
    while !cited && start + height <= lines {
        let hit = matches(start, height);
        windows.push(hit);
        if hit {
            break;
        }
        start += 1;
    }
    let to = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    match place(cited, &windows) {
        Place::Same => Located::Same,
        Place::At(start) => Located::At(to(start + 1), to(start + height)),
        Place::Absent => Located::Absent,
    }
}
