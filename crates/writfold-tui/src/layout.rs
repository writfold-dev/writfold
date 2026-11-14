//! Pure terminal size classes and pane rectangles.
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Wide,
    Medium,
    Narrow,
    TooSmall,
}

pub fn class(width: u16, height: u16) -> Class {
    if width >= 140 && height >= 32 {
        Class::Wide
    } else if width >= 90 && height >= 16 {
        Class::Medium
    } else if width >= 60 && height >= 12 {
        Class::Narrow
    } else {
        Class::TooSmall
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Areas {
    pub header: Rect,
    pub tabs: Rect,
    pub summary: Option<Rect>,
    pub runs: Option<Rect>,
    pub main: Rect,
    pub footer: Rect,
    pub gates: Option<Rect>,
    pub inbox: Option<Rect>,
    pub guards: Option<Rect>,
    pub timeline: Option<Rect>,
}

pub fn areas(width: u16, height: u16) -> Areas {
    let class = class(width, height);
    let mut areas = Areas {
        main: Rect::new(0, 0, width, height),
        ..Areas::default()
    };
    if class == Class::TooSmall {
        return areas;
    }
    areas.header = Rect::new(0, 0, width, 1);
    areas.tabs = Rect::new(0, 1, width, 1);
    areas.footer = Rect::new(0, height - 1, width, 1);
    let top = if class == Class::Medium { 3 } else { 2 };
    areas.main = Rect::new(0, top, width, height - top - 1);
    match class {
        Class::Wide => wide(&mut areas, width, height),
        Class::Medium => {
            areas.summary = Some(Rect::new(0, 2, width, 1));
            areas.runs = Some(Rect::new(0, top, 30, areas.main.height));
            areas.main.x = 30;
            areas.main.width -= 30;
        }
        Class::Narrow | Class::TooSmall => {}
    }
    areas
}

fn wide(areas: &mut Areas, width: u16, height: u16) {
    areas.runs = Some(Rect::new(0, 2, 40, areas.main.height));
    areas.main.x = 40;
    areas.main.width -= 88;
    let side = width - 48;
    areas.gates = Some(Rect::new(side, 2, 48, 9));
    areas.inbox = Some(Rect::new(side, 11, 48, 9));
    let top = if height >= 38 {
        areas.guards = Some(Rect::new(side, 20, 48, 7));
        27
    } else {
        20
    };
    areas.timeline = Some(Rect::new(side, top, 48, height - top - 1));
}

#[cfg(test)]
mod tests {
    use super::{Class, areas, class};
    use ratatui::layout::Rect;

    #[test]
    fn class_boundaries() {
        for (width, height, expected) in [
            (139, 32, Class::Medium),
            (140, 32, Class::Wide),
            (140, 31, Class::Medium),
            (140, 37, Class::Wide),
            (140, 38, Class::Wide),
            (89, 32, Class::Narrow),
            (90, 32, Class::Medium),
            (90, 15, Class::Narrow),
            (90, 16, Class::Medium),
            (59, 12, Class::TooSmall),
            (60, 12, Class::Narrow),
            (60, 11, Class::TooSmall),
            (140, 11, Class::TooSmall),
            (59, 38, Class::TooSmall),
            (0, 0, Class::TooSmall),
        ] {
            assert_eq!(class(width, height), expected, "{width}x{height}");
        }
    }

    #[test]
    fn wide_rectangles_reserve_the_guard_height() {
        let small = areas(140, 32);
        assert_eq!(small.header, Rect::new(0, 0, 140, 1));
        assert_eq!(small.tabs, Rect::new(0, 1, 140, 1));
        assert_eq!(small.runs, Some(Rect::new(0, 2, 40, 29)));
        assert_eq!(small.main, Rect::new(40, 2, 52, 29));
        assert_eq!(small.gates, Some(Rect::new(92, 2, 48, 9)));
        assert_eq!(small.inbox, Some(Rect::new(92, 11, 48, 9)));
        assert_eq!(small.guards, None);
        assert_eq!(small.timeline, Some(Rect::new(92, 20, 48, 11)));
        assert_eq!(small.footer, Rect::new(0, 31, 140, 1));
        assert_eq!(areas(140, 37).guards, None);
        assert_eq!(areas(140, 37).timeline, Some(Rect::new(92, 20, 48, 16)));
        let tall = areas(160, 38);
        assert_eq!(tall.main, Rect::new(40, 2, 72, 35));
        assert_eq!(tall.guards, Some(Rect::new(112, 20, 48, 7)));
        assert_eq!(tall.timeline, Some(Rect::new(112, 27, 48, 10)));
    }

    #[test]
    fn medium_and_narrow_rectangles() {
        let medium = areas(90, 16);
        assert_eq!(medium.header, Rect::new(0, 0, 90, 1));
        assert_eq!(medium.tabs, Rect::new(0, 1, 90, 1));
        assert_eq!(medium.summary, Some(Rect::new(0, 2, 90, 1)));
        assert_eq!(medium.runs, Some(Rect::new(0, 3, 30, 12)));
        assert_eq!(medium.main, Rect::new(30, 3, 60, 12));
        assert_eq!(medium.footer, Rect::new(0, 15, 90, 1));
        assert_eq!(medium.timeline, None);
        let narrow = areas(60, 12);
        assert_eq!(narrow.header, Rect::new(0, 0, 60, 1));
        assert_eq!(narrow.tabs, Rect::new(0, 1, 60, 1));
        assert_eq!(narrow.main, Rect::new(0, 2, 60, 9));
        assert_eq!(narrow.footer, Rect::new(0, 11, 60, 1));
        assert_eq!(narrow.summary, None);
        assert_eq!(narrow.runs, None);
        assert_eq!(narrow.gates, None);
    }

    #[test]
    fn every_rectangle_stays_in_its_frame() {
        for width in [0, 59, 60, 89, 90, 139, 140, u16::MAX] {
            for height in [0, 11, 12, 15, 16, 31, 32, 37, 38, u16::MAX] {
                let a = areas(width, height);
                let all = [
                    Some(a.header),
                    Some(a.tabs),
                    a.summary,
                    a.runs,
                    Some(a.main),
                    Some(a.footer),
                    a.gates,
                    a.inbox,
                    a.guards,
                    a.timeline,
                ];
                for rect in all.into_iter().flatten() {
                    assert!(u32::from(rect.x) + u32::from(rect.width) <= u32::from(width));
                    assert!(u32::from(rect.y) + u32::from(rect.height) <= u32::from(height));
                }
            }
        }
    }
}
