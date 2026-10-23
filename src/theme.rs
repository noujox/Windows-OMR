//! Native libadwaita colors; no Omarchy palette or theme watcher.
//! Follows Windows or Linux system preferences automatically.

pub type Rgb = (f64, f64, f64);

/// Keep existing waveform and animation fallback colors.
pub fn color(_name: &str, fallback: Rgb) -> Rgb {
    fallback
}

pub fn mix(a: Rgb, b: Rgb, amount: f64) -> Rgb {
    (
        a.0 + (b.0 - a.0) * amount,
        a.1 + (b.1 - a.1) * amount,
        a.2 + (b.2 - a.2) * amount,
    )
}

/// Refresh custom-drawn widgets on system theme changes.
pub fn follow(changed: impl Fn() + 'static) {
    adw::StyleManager::default().connect_dark_notify(move |_| changed());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_fallback_colors() {
        let blue = (0.21, 0.52, 0.89);
        assert_eq!(color("blue", blue), blue);
        assert_eq!(mix((0.0, 0.0, 0.0), (1.0, 1.0, 1.0), 0.5), (0.5, 0.5, 0.5));
    }
}
