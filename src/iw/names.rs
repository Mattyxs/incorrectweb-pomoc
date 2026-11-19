//! IW: names people see on Windows (window title, shortcuts, Start menu folder) beside the technical name.
//! The technical name (`APP_NAME` = IncorrectWebPomoc: exe, service, registry key, config and log folders, update files)
//! stays; what Windows shows in the taskbar, on the desktop and in the Start menu is the product name.
//! Older versions named the shortcuts and the Start menu folder after the technical name: an installation removes them,
//! an update renames them, an uninstallation removes both.

/// Start menu of all users (cmd syntax).
pub const START_MENU_PROGRAMS: &str = "%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs";
const DESKTOP: &str = "%PUBLIC%\\Desktop";

/// Shortcut of the program (desktop, Start menu).
pub fn link(display: &str) -> String {
    format!("{display}.lnk")
}

/// Shortcut that uninstalls (Start menu, installation folder).
pub fn uninstall_link(display: &str) -> String {
    format!("Odinstaluj {display}.lnk")
}

/// Start menu folder of the program.
pub fn start_menu(display: &str) -> String {
    format!("{START_MENU_PROGRAMS}\\{display}")
}

/// Main window title, also used by a second start to find the running window (taskbar shows it).
pub fn window_title(display: &str) -> String {
    display.to_owned()
}

/// cmd lines of an update: shortcuts of an older version (technical name) take the product name, where they exist.
pub fn rename_legacy_shortcuts(app_name: &str, display: &str) -> String {
    let old_menu = start_menu(app_name);
    let new_menu = start_menu(display);
    let old_link = format!("{app_name}.lnk");
    let old_uninstall = format!("Uninstall {app_name}.lnk");
    let (link, uninstall) = (link(display), uninstall_link(display));
    format!(
        "if exist \"{DESKTOP}\\{old_link}\" move /Y \"{DESKTOP}\\{old_link}\" \"{DESKTOP}\\{link}\"
if exist \"{old_menu}\\{old_link}\" move /Y \"{old_menu}\\{old_link}\" \"{old_menu}\\{link}\"
if exist \"{old_menu}\\{old_uninstall}\" move /Y \"{old_menu}\\{old_uninstall}\" \"{old_menu}\\{uninstall}\"
if exist \"{old_menu}\" if not exist \"{new_menu}\" move /Y \"{old_menu}\" \"{new_menu}\"
"
    )
}

/// cmd lines of an uninstallation (and of the installation, which starts with it): shortcuts and Start menu folders
/// under both names.
pub fn remove_shortcuts(app_name: &str, display: &str) -> String {
    let mut out = String::new();
    for name in [app_name, display] {
        let menu = start_menu(name);
        let link = link(name);
        out.push_str(&format!(
            "if exist \"{menu}\" rd /s /q \"{menu}\"\nif exist \"{DESKTOP}\\{link}\" del /f /q \"{DESKTOP}\\{link}\"\n"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: &str = "IncorrectWebPomoc";
    const SHOWN: &str = "GoConnect Pomoc";

    #[test]
    fn shortcuts_and_folder_carry_the_product_name() {
        assert_eq!(link(SHOWN), "GoConnect Pomoc.lnk");
        assert_eq!(uninstall_link(SHOWN), "Odinstaluj GoConnect Pomoc.lnk");
        assert_eq!(start_menu(SHOWN), "%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\GoConnect Pomoc");
        assert_eq!(window_title(SHOWN), "GoConnect Pomoc");
    }

    #[test]
    fn an_update_renames_the_shortcuts_of_an_older_version() {
        let s = rename_legacy_shortcuts(APP, SHOWN);
        assert!(s.contains("if exist \"%PUBLIC%\\Desktop\\IncorrectWebPomoc.lnk\" move /Y \"%PUBLIC%\\Desktop\\IncorrectWebPomoc.lnk\" \"%PUBLIC%\\Desktop\\GoConnect Pomoc.lnk\""));
        assert!(s.contains("Programs\\IncorrectWebPomoc\\Uninstall IncorrectWebPomoc.lnk\" \"%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\IncorrectWebPomoc\\Odinstaluj GoConnect Pomoc.lnk\""));
        // the folder last, after the links in it got their names; never over an existing folder
        let folder = s.lines().last().unwrap();
        assert_eq!(folder, "if exist \"%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\IncorrectWebPomoc\" if not exist \"%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\GoConnect Pomoc\" move /Y \"%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\IncorrectWebPomoc\" \"%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\GoConnect Pomoc\"");
        // every path is quoted (names with a space)
        for line in s.lines() {
            assert_eq!(line.matches('"').count() % 2, 0, "{line}");
        }
    }

    #[test]
    fn an_uninstallation_removes_both_names() {
        let s = remove_shortcuts(APP, SHOWN);
        for name in [APP, SHOWN] {
            assert!(s.contains(&format!("rd /s /q \"%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\\{name}\"")));
            assert!(s.contains(&format!("del /f /q \"%PUBLIC%\\Desktop\\{name}.lnk\"")));
        }
        assert!(!s.contains("rd /s /q \"%ProgramData%\\Microsoft\\Windows\\Start Menu\\Programs\"\n"), "never the Programs folder itself");
    }
}
