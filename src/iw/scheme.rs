//! IW: when this copy of the program may (re)write the HKCU registration of `iwpomoc://`.

/// True when `exe` sits in the installation directory, or when no registration exists yet.
/// A copy started from Downloads or a temporary folder must not take over an existing registration.
pub fn should_register(exe: &str, install_dir: &str, existing_command: Option<&str>) -> bool {
    in_directory(exe, install_dir) || existing_command.map_or(true, |c| c.trim().is_empty())
}

fn parts(path: &str) -> Vec<String> {
    path.split(['/', '\\'])
        .filter(|p| !p.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// `exe` is a file below `dir` (Windows rules: both separators, case-insensitive, no `..`).
fn in_directory(exe: &str, dir: &str) -> bool {
    let (exe, dir) = (parts(exe), parts(dir));
    !dir.is_empty()
        && !exe.iter().any(|p| p == "..")
        && !dir.iter().any(|p| p == "..")
        && exe.len() > dir.len()
        && exe[..dir.len()] == dir[..]
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIR: &str = r"C:\Program Files\IncorrectWebPomoc";
    const OLD: Option<&str> = Some(r#""C:\Program Files\IncorrectWebPomoc\IncorrectWebPomoc.exe" "%1""#);

    #[test]
    fn registers_from_the_install_directory_even_over_an_existing_key() {
        assert!(should_register(r"C:\Program Files\IncorrectWebPomoc\IncorrectWebPomoc.exe", DIR, OLD));
        assert!(should_register(r"c:\program files\incorrectwebpomoc\IncorrectWebPomoc.exe", DIR, OLD));
        assert!(should_register("C:/Program Files/IncorrectWebPomoc/IncorrectWebPomoc.exe", r"C:\Program Files\IncorrectWebPomoc\", OLD));
    }

    #[test]
    fn does_not_overwrite_an_existing_key_from_downloads_or_temp() {
        assert!(!should_register(r"C:\Users\Jan\Downloads\IncorrectWebPomoc-ABCD-EFGH.exe", DIR, OLD));
        assert!(!should_register(r"C:\Users\Jan\AppData\Local\Temp\IncorrectWebPomoc\IncorrectWebPomoc.exe", DIR, OLD));
        assert!(!should_register(r"C:\Users\Jan\Downloads\p.exe", DIR, Some(r#""D:\x\p.exe" "%1""#)));
    }

    #[test]
    fn registers_from_anywhere_when_the_key_is_missing() {
        assert!(should_register(r"C:\Users\Jan\Downloads\p.exe", DIR, None));
        assert!(should_register(r"C:\Users\Jan\Downloads\p.exe", DIR, Some("")));
        assert!(should_register(r"C:\Users\Jan\Downloads\p.exe", DIR, Some("  ")));
    }

    #[test]
    fn lookalike_and_traversal_paths_are_not_the_install_directory() {
        assert!(!should_register(r"C:\Program Files\IncorrectWebPomoc-evil\p.exe", DIR, OLD));
        assert!(!should_register(r"C:\Program Files\IncorrectWebPomoc\..\..\Users\Jan\Downloads\p.exe", DIR, OLD));
        assert!(!should_register(r"C:\Program Files\IncorrectWebPomoc", DIR, OLD));
    }

    #[test]
    fn empty_install_directory_matches_nothing() {
        assert!(!should_register(r"C:\x\p.exe", "", OLD));
        assert!(!should_register(r"\x\p.exe", "", OLD));
    }
}
