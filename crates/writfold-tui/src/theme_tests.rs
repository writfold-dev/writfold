#[cfg(test)]
mod tests {
    use crate::theme::{Palette, Rgb, ThemeId};

    const TOLERANCE: f64 = 1e-9;

    fn luminance(color: Rgb) -> f64 {
        fn linear(component: u8) -> f64 {
            let value = f64::from(component) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(color.0) + 0.7152 * linear(color.1) + 0.0722 * linear(color.2)
    }

    fn contrast(first: Rgb, second: Rgb) -> f64 {
        let first = luminance(first);
        let second = luminance(second);
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }

    fn at_least(id: ThemeId, role: &str, ink: Rgb, background: Rgb, floor: f64) {
        let ratio = contrast(ink, background);
        assert!(
            ratio + TOLERANCE >= floor,
            "{} {role}: {ratio:.12}:1 < {floor}:1",
            id.name()
        );
    }

    fn inks(palette: &Palette) -> [(&'static str, Rgb, f64); 9] {
        [
            ("text", palette.text, 10.0),
            ("secondary", palette.secondary, 7.0),
            ("muted", palette.muted, 4.5),
            ("accent", palette.accent, 4.5),
            ("error", palette.error, 4.5),
            ("warning", palette.warning, 4.5),
            ("info", palette.info, 4.5),
            ("good", palette.good, 4.5),
            ("vouched", palette.vouched, 4.5),
        ]
    }

    #[test]
    fn inks_clear_their_floors_on_base_and_surface() {
        for id in ThemeId::ALL {
            let palette = id.palette();
            for background in [palette.base, palette.surface] {
                for (role, ink, floor) in inks(palette) {
                    at_least(id, role, ink, background, floor);
                }
            }
            at_least(id, "on accent", palette.on_accent, palette.accent, 4.5);
            at_least(id, "key", palette.accent, palette.raised, 4.5);
        }
    }

    #[test]
    fn tonal_ladder_has_visible_steps() {
        for id in ThemeId::ALL {
            let palette = id.palette();
            for (step, first, second) in [
                ("base to surface", palette.base, palette.surface),
                ("surface to raised", palette.surface, palette.raised),
            ] {
                let ratio = contrast(first, second);
                assert!(
                    (1.07 - TOLERANCE..=1.30 + TOLERANCE).contains(&ratio),
                    "{} {step}: {ratio:.12}:1 outside 1.07–1.30",
                    id.name()
                );
            }
            at_least(id, "raised over base", palette.raised, palette.base, 1.18);
        }
    }

    #[test]
    fn selection_is_visible_and_its_text_is_readable() {
        for id in ThemeId::ALL {
            let palette = id.palette();
            let floor = if id.is_light() {
                1.45
            } else if id == ThemeId::Slate {
                1.49
            } else {
                1.55
            };
            at_least(id, "selection", palette.selection, palette.base, floor);
            at_least(id, "selected text", palette.text, palette.selection, 7.0);
        }
    }

    #[test]
    fn registry_names_round_trip_and_default_is_hone() {
        let names = ["hone", "slate", "carbide", "vellum", "paper", "paper-dark"];
        assert_eq!(ThemeId::default(), ThemeId::Hone);
        assert_eq!(ThemeId::choices(), names.join(", "));
        for (id, name) in ThemeId::ALL.into_iter().zip(names) {
            assert_eq!(id.name(), name);
            assert_eq!(ThemeId::from_name(name), Some(id));
            assert_eq!(ThemeId::from_name(&name.to_ascii_uppercase()), Some(id));
            assert_eq!(ThemeId::from_name(&format!(" \t{name}\n")), Some(id));
            assert_eq!(
                ThemeId::from_name(&format!(" \t{}\n", name.to_ascii_uppercase())),
                Some(id)
            );
            assert_eq!(
                id.is_light(),
                matches!(id, ThemeId::Vellum | ThemeId::Paper)
            );
        }
        for name in ["nord", "dark", "", " \t\n", "paper dark", "hone-extra"] {
            assert_eq!(ThemeId::from_name(name), None);
        }
    }

    #[test]
    fn next_walks_registry_order_and_wraps() {
        let mut current = ThemeId::default();
        for id in ThemeId::ALL {
            assert_eq!(current, id);
            current = current.next();
        }
        assert_eq!(current, ThemeId::Hone);
    }
}
