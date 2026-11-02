//! Writing text to the system clipboard through the platform's own tools (no extra dependencies).

use std::io::Write;
use std::process::{Command, Stdio};

pub fn clipboard_text() -> Option<String> {
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new("powershell");
        command.args(["-NoProfile", "-NonInteractive", "-Command", "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-Clipboard -Raw"]).creation_flags(0x0800_0000);
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("wl-paste");
        command.arg("--no-newline");
        command
    };
    #[cfg(not(any(windows, target_os = "linux")))]
    let mut command = Command::new("pbpaste");
    let output = command.stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    output.status.success().then(|| String::from_utf8(output.stdout).ok()).flatten()
}

/// Puts `text` on the clipboard; runs off-thread because the helper process takes a moment to start.
pub fn set_clipboard(text: String) {
    std::thread::spawn(move || {
        #[cfg(windows)]
        let mut cmd = {
            use std::os::windows::process::CommandExt;
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-NonInteractive", "-Command", "[Console]::InputEncoding=[Text.Encoding]::UTF8; Set-Clipboard -Value ([Console]::In.ReadToEnd())"]).creation_flags(0x0800_0000);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = Command::new("wl-copy");
        let Ok(mut child) = cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else { return };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    });
}
