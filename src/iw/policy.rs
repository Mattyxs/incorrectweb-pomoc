//! IW: safe defaults decided in one place, so tests can pin them.

/// Value of the `enable-lan-discovery` option when nobody set it.
pub const DEFAULT_LAN_DISCOVERY: &str = "N";

/// Display options of a computer nobody has connected to yet: fit the image to the window,
/// show the cursor of the client. Written to `DEFAULT_DISPLAY_SETTINGS`, so a saved choice wins.
pub const DISPLAY_DEFAULTS: &[(&str, &str)] =
    &[("view_style", "adaptive"), ("show_remote_cursor", "Y"), ("scroll_style", "scrollauto")];

/// RustDesk starts desktop sessions with `original` (pixel for pixel), which on a Retina screen
/// shows a quarter of the picture or a part of it that scrolls.
pub fn view_style_default() -> &'static str {
    "adaptive"
}

/// LAN discovery answers pings of other computers only after an explicit opt-in.
pub fn lan_discovery_enabled(option_value: &str) -> bool {
    option_value == "Y"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_defaults_fit_window_and_show_client_cursor() {
        let get = |k: &str| DISPLAY_DEFAULTS.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
        assert_eq!(get("view_style"), Some("adaptive"));
        assert_eq!(get("show_remote_cursor"), Some("Y"));
        assert_eq!(view_style_default(), "adaptive");
    }

    #[test]
    fn a_new_peer_gets_the_fitted_view_after_the_defaults_are_installed() {
        {
            let mut d = hbb_common::config::DEFAULT_DISPLAY_SETTINGS.write().unwrap();
            for (k, v) in DISPLAY_DEFAULTS {
                d.entry((*k).to_owned()).or_insert_with(|| (*v).to_owned());
            }
        }
        assert_eq!(hbb_common::config::PeerConfig::default().view_style, "adaptive");
    }

    #[test]
    fn lan_discovery_is_off_unless_explicitly_enabled() {
        assert!(!lan_discovery_enabled(""));
        assert!(!lan_discovery_enabled("N"));
        assert!(!lan_discovery_enabled("y"));
        assert!(!lan_discovery_enabled("1"));
        assert!(lan_discovery_enabled("Y"));
    }

    #[test]
    fn the_default_option_value_is_off() {
        assert_eq!(DEFAULT_LAN_DISCOVERY, "N");
        assert!(!lan_discovery_enabled(DEFAULT_LAN_DISCOVERY));
    }
}
