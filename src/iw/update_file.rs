//! IW: the downloaded update package on the disk (Windows: the service runs it as SYSTEM). It goes to a folder of its
//! own that only SYSTEM and the administrators may write, is written anew every time, and is kept open read-only,
//! without sharing writes or deletion, from the last check of its signature until the installer has started.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// `<ProgramData>\IncorrectWebPomoc\updates` (no space: the installer command line is not quoted, src/updater.rs).
pub fn updates_dir(program_data: &Path) -> PathBuf {
    program_data.join("IncorrectWebPomoc").join("updates")
}

/// `icacls` calls that leave the folder to SYSTEM and the administrators only (owner SYSTEM, no inherited rights).
pub fn acl_commands(dir: &Path) -> Vec<Vec<String>> {
    let d = dir.to_string_lossy().to_string();
    let s = |v: &str| v.to_owned();
    vec![
        vec![d.clone(), s("/setowner"), s("*S-1-5-18")],
        vec![d, s("/inheritance:r"), s("/grant:r"), s("*S-1-5-18:(OI)(CI)F"), s("/grant:r"), s("*S-1-5-32-544:(OI)(CI)F")],
    ]
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
    fn the_folder_is_under_program_data_without_spaces() {
        let d = updates_dir(Path::new(r"C:\ProgramData"));
        assert_eq!(d, Path::new(r"C:\ProgramData").join("IncorrectWebPomoc").join("updates"));
        assert!(!d.to_string_lossy().contains(' '));
    }

    #[test]
    fn only_system_and_the_administrators_keep_rights() {
        let d = Path::new(r"C:\ProgramData\IncorrectWebPomoc\updates");
        let cmds = acl_commands(d);
        let p = d.to_string_lossy().to_string();
        assert_eq!(
            cmds,
            vec![
                vec![p.clone(), "/setowner".into(), "*S-1-5-18".into()],
                vec![p, "/inheritance:r".into(), "/grant:r".into(), "*S-1-5-18:(OI)(CI)F".into(), "/grant:r".into(), "*S-1-5-32-544:(OI)(CI)F".into()],
            ]
        );
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
