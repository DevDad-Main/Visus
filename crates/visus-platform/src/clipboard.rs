//! Writing text to the system clipboard through the platform's own tools (no extra dependencies).

use std::io::Write;
use std::process::{Command, Stdio};

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
