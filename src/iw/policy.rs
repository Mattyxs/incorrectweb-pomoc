//! IW: safe defaults decided in one place, so tests can pin them.

/// Value of the `enable-lan-discovery` option when nobody set it.
pub const DEFAULT_LAN_DISCOVERY: &str = "N";

/// LAN discovery answers pings of other computers only after an explicit opt-in.
pub fn lan_discovery_enabled(option_value: &str) -> bool {
    option_value == "Y"
}

#[cfg(test)]
mod tests {
    use super::*;

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
