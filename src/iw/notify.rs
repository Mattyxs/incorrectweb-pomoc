//! IW: system notification commands. Texts (they contain a person's name) never become part of a script:
//! PowerShell gets one fixed script as `-EncodedCommand` and reads the texts from environment variables,
//! osascript gets them as `argv`.

use hbb_common::base64::{engine::general_purpose::STANDARD, Engine};

pub const ENV_TITLE: &str = "IW_TOAST_TITLE";
pub const ENV_TEXT: &str = "IW_TOAST_TEXT";

/// The one script that is ever run. It must stay free of any interpolated text.
pub const TOAST_SCRIPT: &str = "\
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
$x = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
$t = $x.GetElementsByTagName('text'); \
$t.Item(0).AppendChild($x.CreateTextNode($env:IW_TOAST_TITLE)) > $null; \
$t.Item(1).AppendChild($x.CreateTextNode($env:IW_TOAST_TEXT)) > $null; \
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($env:IW_TOAST_TITLE).Show([Windows.UI.Notifications.ToastNotification]::new($x))";

pub struct PowerShellCall {
    pub args: Vec<String>,
    pub env: Vec<(&'static str, String)>,
}

/// `-EncodedCommand` wants Base64 of the UTF-16LE script.
pub fn encode_command(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    STANDARD.encode(bytes)
}

/// A NUL cannot live in an environment variable or an argument; the text is shortened to a notification size.
fn clean(s: &str) -> String {
    s.chars().filter(|c| *c != '\0').take(400).collect()
}

pub fn powershell_toast(title: &str, text: &str) -> PowerShellCall {
    PowerShellCall {
        args: ["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-EncodedCommand"]
            .iter()
            .map(|a| (*a).to_owned())
            .chain([encode_command(TOAST_SCRIPT)])
            .collect(),
        env: vec![(ENV_TITLE, clean(title)), (ENV_TEXT, clean(text))],
    }
}

pub fn osascript_args(title: &str, text: &str) -> Vec<String> {
    [
        "-e",
        "on run argv",
        "-e",
        "display notification (item 1 of argv) with title (item 2 of argv)",
        "-e",
        "end run",
    ]
    .iter()
    .map(|a| (*a).to_owned())
    .chain([clean(text), clean(title)])
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOSTILE: &str = "x'; Start-Process calc; \"$(whoami)` $env:USERNAME ’ ” \\ \n \u{0}end";

    fn decode(enc: &str) -> String {
        let bytes = STANDARD.decode(enc).unwrap();
        let units: Vec<u16> = bytes.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16(&units).unwrap()
    }

    fn encoded_arg(call: &PowerShellCall) -> &str {
        let i = call.args.iter().position(|a| a == "-EncodedCommand").expect("-EncodedCommand");
        &call.args[i + 1]
    }

    #[test]
    fn encode_command_is_base64_of_utf16le() {
        assert_eq!(encode_command("ab"), "YQBiAA==");
        assert_eq!(decode(&encode_command("zażółć")), "zażółć");
    }

    #[test]
    fn powershell_runs_one_fixed_script_whatever_the_text() {
        let a = powershell_toast("IncorrectWeb Pomoc", "Jan");
        let b = powershell_toast("IncorrectWeb Pomoc", HOSTILE);
        assert_eq!(a.args, b.args);
        assert_eq!(decode(encoded_arg(&b)), TOAST_SCRIPT);
        assert!(!a.args.iter().any(|x| x == "-Command"), "no -Command with a script text");
    }

    #[test]
    fn powershell_script_reads_texts_from_environment() {
        assert!(TOAST_SCRIPT.contains("$env:IW_TOAST_TITLE"));
        assert!(TOAST_SCRIPT.contains("$env:IW_TOAST_TEXT"));
        assert!(TOAST_SCRIPT.contains("CreateTextNode($env:IW_TOAST_TEXT)"));
        // no quoted literal in a text node, nothing to break out of
        assert!(!TOAST_SCRIPT.contains("CreateTextNode('"));
    }

    #[test]
    fn powershell_texts_travel_in_environment_without_nul() {
        let c = powershell_toast("Tytuł", HOSTILE);
        let get = |k: &str| c.env.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone()).unwrap();
        assert_eq!(get(ENV_TITLE), "Tytuł");
        let text = get(ENV_TEXT);
        assert!(!text.contains('\0'));
        assert!(text.contains("Start-Process calc"));
        assert!(text.contains("$(whoami)"));
        assert!(text.contains('’') && text.contains('”') && text.contains('`'));
    }

    #[test]
    fn osascript_gets_texts_as_argv_not_in_the_script() {
        let a = osascript_args("IncorrectWeb Pomoc", HOSTILE);
        let n = a.len();
        assert_eq!(a[n - 2], HOSTILE.replace('\0', ""));
        assert_eq!(a[n - 1], "IncorrectWeb Pomoc");
        for arg in &a[..n - 2] {
            assert!(!arg.contains("calc") && !arg.contains("whoami"), "{arg}");
        }
        assert!(a.iter().any(|x| x.contains("argv")));
    }
}
