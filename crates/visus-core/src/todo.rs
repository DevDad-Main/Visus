use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub text: String,
    pub done: bool,
    /// Minutes after local midnight for today's reminder.
    pub due: Option<u16>,
}

/// Turns typed text into a task. A trailing `@14:30` (or `@9`) sets a reminder for that time today.
pub fn parse_task(input: &str) -> Option<Todo> {
    let input = input.trim();
    let mut text = input;
    let mut due = None;
    if let Some((head, tail)) = input.rsplit_once('@') {
        if let Some(m) = parse_time(tail.trim()) {
            text = head.trim();
            due = Some(m);
        }
    }
    (!text.is_empty()).then(|| Todo { text: text.to_string(), done: false, due })
}

fn parse_time(s: &str) -> Option<u16> {
    let (h, m) = s.split_once(':').unwrap_or((s, "0"));
    let (h, m): (u16, u16) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reminders() {
        let t = parse_task("Call Dina @14:30").unwrap();
        assert_eq!((t.text.as_str(), t.due), ("Call Dina", Some(870)));
        assert_eq!(parse_task("Stretch @9").unwrap().due, Some(540));
        assert_eq!(parse_task("email a@b.com").unwrap().due, None);
        assert!(parse_task("  ").is_none());
        assert!(parse_task("@10:00").is_none());
    }
}
