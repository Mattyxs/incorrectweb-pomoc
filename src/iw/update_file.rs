//! IW: the downloaded update package on the disk (Windows: the service runs it as SYSTEM). It goes to a folder of its
//! own that only SYSTEM and the administrators may write, is written anew every time, and is kept open read-only,
//! without sharing writes or deletion, from the last check of its signature until the installer has started.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// `<installation folder of Pomoc>\updates`: Program Files, written only by the administrators and SYSTEM.
pub fn updates_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("updates")
}

/// No level of `path` is a symbolic link or a junction (std reports both through `is_symlink`), so the package cannot be
/// written or run somewhere else.
pub fn check_no_links(path: &Path) -> io::Result<()> {
    for level in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        match std::fs::symlink_metadata(level) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("{} is a link", level.display())));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// The command line of the installer: the path in quotes (the folder may have spaces, `C:\Program Files\…`).
pub fn install_command(exe: &Path) -> String {
    format!("\"{}\" --update", exe.display())
}

/// Removes what is there and writes `bytes` as a new file.
pub fn write_fresh(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Read-only; on Windows others may only read it meanwhile (no writing, no deleting, no renaming).
pub fn open_locked(path: &Path) -> io::Result<File> {
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x1;
        o.share_mode(FILE_SHARE_READ);
    }
    o.open(path)
}

/// The whole content, read through the open handle.
pub fn read_all(file: &mut File) -> io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(0))?;
    let mut v = Vec::new();
    file.read_to_end(&mut v)?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("iw-update-file-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_folder_is_in_the_installation_folder() {
        let d = updates_dir(Path::new(r"C:\Program Files\IncorrectWebPomoc"));
        assert_eq!(d, Path::new(r"C:\Program Files\IncorrectWebPomoc").join("updates"));
    }

    #[test]
    fn the_installer_path_is_quoted() {
        let exe = Path::new("C:/Program Files/IncorrectWebPomoc/updates/IncorrectWebPomoc-1.4.9-iw.7-x64.exe");
        assert_eq!(install_command(exe), "\"C:/Program Files/IncorrectWebPomoc/updates/IncorrectWebPomoc-1.4.9-iw.7-x64.exe\" --update");
    }

    #[test]
    fn a_link_on_any_level_is_refused() {
        let d = std::fs::canonicalize(dir("links")).unwrap();
        let real = d.join("real");
        std::fs::create_dir_all(real.join("updates")).unwrap();
        assert!(check_no_links(&real.join("updates")).is_ok());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real, d.join("link")).unwrap();
            assert!(check_no_links(&d.join("link").join("updates")).is_err(), "a linked parent");
            std::fs::write(real.join("file.exe"), b"x").unwrap();
            std::os::unix::fs::symlink(real.join("file.exe"), real.join("updates").join("p.exe")).unwrap();
            assert!(check_no_links(&real.join("updates").join("p.exe")).is_err(), "a linked file");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_file_left_from_before_is_replaced() {
        let d = dir("fresh");
        let f = d.join("IncorrectWebPomoc-1.4.9-iw.7-x64.exe");
        std::fs::write(&f, b"left by someone, longer than the package").unwrap();
        write_fresh(&f, b"package").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"package");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_open_file_is_read_through_its_handle() {
        let d = dir("locked");
        let f = d.join("p.exe");
        write_fresh(&f, b"package").unwrap();
        let mut h = open_locked(&f).unwrap();
        assert_eq!(read_all(&mut h).unwrap(), b"package");
        assert_eq!(read_all(&mut h).unwrap(), b"package", "read again from the start");
        let _ = std::fs::remove_dir_all(&d);
    }
}
