//! Quick launcher / command palette: search state, ranking and a tiny calculator.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::Page;

pub const MAX_RESULTS: usize = 5;

/// Rough category used to pick a fallback icon when an app has none of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppKind {
    #[default]
    Generic,
    Game,
    Browser,
    Terminal,
    Music,
    Video,
    Code,
    Office,
    Files,
    Settings,
    Chat,
    Graphics,
}

/// Guesses an app's category from freedesktop `Categories=` (may be empty) and its name.
pub fn guess_kind(name: &str, categories: &str) -> AppKind {
    let c = categories.to_lowercase();
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    let cat = |words: &[&str]| words.iter().any(|w| c.contains(w));
    if cat(&["game"]) || has(&["steam", "game", "epic", "battle.net", "riot", "minecraft", "launcher"]) {
        AppKind::Game
    } else if cat(&["webbrowser"]) || has(&["chrome", "firefox", "edge", "brave", "opera", "browser", "vivaldi"]) {
        AppKind::Browser
    } else if cat(&["terminalemulator"]) || has(&["terminal", "powershell", "cmd", "konsole", "alacritty", "wezterm", "kitty"]) {
        AppKind::Terminal
    } else if cat(&["instantmessaging", "chat"]) || has(&["discord", "telegram", "slack", "teams", "whatsapp", "signal"]) {
        AppKind::Chat
    } else if cat(&["video"]) || has(&["vlc", "mpv", "video", "movies", "netflix"]) {
        AppKind::Video
    } else if cat(&["audio", "music"]) || has(&["spotify", "music", "foobar", "audacity"]) {
        AppKind::Music
    } else if cat(&["development", "texteditor"]) || has(&["code", "studio", "vim", "intellij", "pycharm", "rider", "sublime", "notepad"]) {
        AppKind::Code
    } else if cat(&["office"]) || has(&["word", "excel", "powerpoint", "office", "onenote", "outlook"]) {
        AppKind::Office
    } else if cat(&["filemanager"]) || has(&["explorer", "nautilus", "dolphin", "thunar"]) {
        AppKind::Files
    } else if cat(&["settings", "system"]) || has(&["settings", "control panel"]) {
        AppKind::Settings
    } else if cat(&["graphics"]) || has(&["photoshop", "gimp", "krita", "blender", "figma", "inkscape", "paint"]) {
        AppKind::Graphics
    } else {
        AppKind::Generic
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct App {
    pub name: String,
    /// Icon theme name or file path (Linux); empty when the platform finds it from `target`.
    pub icon: String,
    pub kind: AppKind,
    /// What the platform needs to start it (shortcut path on Windows, command line on Linux).
    pub target: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    ShowPage(Page),
    OpenSettings,
    ToggleTimer,
    ResetTimer,
    ToggleMinimal,
    ToggleEye,
    Screenshot,
    CheckUpdates,
    Quit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Launch(String),
    Command(Command),
    OpenUrl(String),
    AddNote(String),
    SetCalendar(String),
    FocusWindow(isize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    App,
    Visus,
    Calc,
    Web,
    Window,
    Note,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub title: String,
    pub subtitle: String,
    pub kind: Kind,
    /// None for informational rows such as a calculator result.
    pub action: Option<Action>,
    /// Set for application rows so the icon can be looked up.
    pub app: Option<App>,
}

/// What the host must do after the user confirms a result.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    None,
    Geocode(String),
    Launch(String),
    OpenUrl(String),
    OpenPath(String),
    /// App folders changed; rebuild the launcher's app list.
    Rescan,
    Screenshot,
    CheckUpdates,
    InstallUpdate,
    FocusWindow(isize),
    Quit,
}

#[derive(Clone, Debug, Default)]
pub struct Palette {
    pub query: String,
    pub selected: usize,
    pub results: Vec<Item>,
    apps: Arc<Vec<App>>,
    windows: Vec<(isize, String)>,
    icons: HashMap<String, Option<Arc<crate::Art>>>,
    requested: HashSet<String>,
}

fn visus_commands() -> Vec<(&'static str, &'static str, Command)> {
    vec![
        ("Open Music", "Show the music page", Command::ShowPage(Page::Music)),
        ("Open Control Center", "Volume, Wi-Fi, Bluetooth, quiet mode", Command::ShowPage(Page::Control)),
        ("Open Timer", "Show the focus timer", Command::ShowPage(Page::Timer)),
        ("Open Notes", "Quick notes: type note <text>", Command::ShowPage(Page::Notes)),
        ("Open Clipboard", "Recent copied text", Command::ShowPage(Page::Clipboard)),
        ("Open Stats", "Show CPU, memory and network", Command::ShowPage(Page::Stats)),
        ("Open To-do", "Show your tasks", Command::ShowPage(Page::Todo)),
        ("Open Shelf", "Show shelved files", Command::ShowPage(Page::Shelf)),
        ("Open Calendar", "Show this month", Command::ShowPage(Page::Calendar)),
        ("Open Weather", "Show current conditions", Command::ShowPage(Page::Weather)),
        ("Open Settings", "Sizes, eye, updates", Command::OpenSettings),
        ("Take screenshot", "Select an area to capture", Command::Screenshot),
        ("Check for updates", "Look for a newer Visus", Command::CheckUpdates),
        ("Start or pause focus timer", "Pomodoro", Command::ToggleTimer),
        ("Reset focus timer", "Pomodoro", Command::ResetTimer),
        ("Toggle minimal notch", "Thin line when closed", Command::ToggleMinimal),
        ("Toggle idle eye", "Show or hide the eye", Command::ToggleEye),
        ("Quit Visus", "Close the app", Command::Quit),
    ]
}

impl Palette {
    /// A loaded icon for an app result, if one was found.
    pub fn icon_for(&self, item: &Item) -> Option<&Arc<crate::Art>> {
        let app = item.app.as_ref()?;
        self.icons.get(&app.target)?.as_ref()
    }

    pub fn set_icon(&mut self, target: String, art: Option<Arc<crate::Art>>) {
        self.icons.insert(target, art);
    }

    /// Apps in the current results whose icons have not been requested yet.
    pub fn take_icon_requests(&mut self) -> Vec<App> {
        let mut out = Vec::new();
        for app in self.results.iter().filter_map(|i| i.app.as_ref()) {
            if !self.icons.contains_key(&app.target) && self.requested.insert(app.target.clone()) {
                out.push(app.clone());
            }
        }
        out
    }

    pub fn set_windows(&mut self, w: Vec<(isize, String)>) {
        self.windows = w;
        self.refresh();
    }

    pub fn set_apps(&mut self, apps: Arc<Vec<App>>) {
        self.apps = apps;
        self.refresh();
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.refresh();
    }

    pub fn type_text(&mut self, s: &str) {
        self.query.extend(s.chars().filter(|c| !c.is_control()));
        self.refresh();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.refresh();
    }

    pub fn move_selection(&mut self, delta: i32) {
        let n = self.results.len() as i32;
        if n > 0 {
            self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
        }
    }

    pub fn selected_item(&self) -> Option<&Item> {
        self.results.get(self.selected)
    }

    pub fn refresh(&mut self) {
        self.results = search_with(&self.query, &self.apps, &self.windows);
        self.selected = 0;
    }
}

/// Higher is better; None when `q` does not match `name` at all. Both must be lowercase.
fn score(name: &str, q: &str) -> Option<i32> {
    if name == q {
        return Some(1000);
    }
    if name.starts_with(q) {
        return Some(800 - name.len() as i32 / 4);
    }
    if name.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(q)) {
        return Some(600 - name.len() as i32 / 4);
    }
    if name.contains(q) {
        return Some(400 - name.len() as i32 / 4);
    }
    // Subsequence match, e.g. "ffx" -> "firefox"; tighter spans score higher.
    let mut it = name.char_indices();
    let (mut first, mut last) = (None, 0);
    for qc in q.chars() {
        let (i, _) = it.find(|(_, c)| *c == qc)?;
        first.get_or_insert(i);
        last = i;
    }
    Some(200 - (last - first.unwrap_or(0)) as i32)
}

#[cfg(test)]
pub fn search(query: &str, apps: &[App]) -> Vec<Item> {
    search_with(query, apps, &[])
}

pub fn search_with(query: &str, apps: &[App], windows: &[(isize, String)]) -> Vec<Item> {
    let q = query.trim();
    let lower = q.to_lowercase();
    let mut out: Vec<Item> = Vec::new();

    if lower.is_empty() {
        return visus_commands()
            .into_iter()
            .take(MAX_RESULTS)
            .map(|(t, s, c)| Item { title: t.into(), subtitle: s.into(), kind: Kind::Visus, action: Some(Action::Command(c)), app: None })
            .collect();
    }

    // `note buy milk` saves a note; `cal <link>` sets the calendar feed.
    if let Some(rest) = lower.strip_prefix("note ").map(|_| q[5..].trim()).filter(|r| !r.is_empty()) {
        out.push(Item { title: "Save note".into(), subtitle: rest.chars().take(80).collect(), kind: Kind::Note, action: Some(Action::AddNote(rest.to_string())), app: None });
    }
    if let Some(rest) = lower.strip_prefix("cal ").map(|_| q[4..].trim()).filter(|r| !r.is_empty()) {
        out.push(Item { title: "Use as calendar".into(), subtitle: rest.chars().take(80).collect(), kind: Kind::Visus, action: Some(Action::SetCalendar(rest.to_string())), app: None });
    }
    if let Some(v) = eval_expression(q) {
        out.push(Item { title: format_number(v), subtitle: format!("{q} ="), kind: Kind::Calc, action: None, app: None });
    }
    if looks_like_url(q) {
        let url = if q.contains("://") { q.to_string() } else { format!("https://{q}") };
        out.push(Item { title: format!("Open {q}"), subtitle: "Web address".into(), kind: Kind::Web, action: Some(Action::OpenUrl(url)), app: None });
    }

    let mut scored: Vec<(i32, Item)> = Vec::new();
    for app in apps {
        if let Some(s) = score(&app.name.to_lowercase(), &lower) {
            scored.push((s, Item { title: app.name.clone(), subtitle: "Application".into(), kind: Kind::App, action: Some(Action::Launch(app.target.clone())), app: Some(app.clone()) }));
        }
    }
    if lower.chars().count() >= 2 {
        for (h, title) in windows {
            if let Some(s) = score(&title.to_lowercase(), &lower) {
                scored.push((s - 10, Item { title: title.clone(), subtitle: "Switch to window".into(), kind: Kind::Window, action: Some(Action::FocusWindow(*h)), app: None }));
            }
        }
    }
    for (title, sub, cmd) in visus_commands() {
        if let Some(s) = score(&title.to_lowercase(), &lower) {
            // Slight bias so a Visus command does not bury an exact app match.
            scored.push((s - 20, Item { title: title.into(), subtitle: sub.into(), kind: Kind::Visus, action: Some(Action::Command(cmd)), app: None }));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.title.len().cmp(&b.1.title.len())));
    let best = scored.first().map_or(0, |s| s.0);
    out.extend(scored.into_iter().map(|(_, i)| i));
    out.truncate(MAX_RESULTS);

    if best < 400 && out.len() < MAX_RESULTS {
        let encoded: String = q
            .bytes()
            .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
            .collect();
        out.push(Item {
            title: format!("Search the web for \u{201c}{q}\u{201d}"),
            subtitle: "DuckDuckGo".into(),
            kind: Kind::Web,
            action: Some(Action::OpenUrl(format!("https://duckduckgo.com/?q={encoded}"))),
            app: None,
        });
    }
    out.truncate(MAX_RESULTS);
    out
}

fn looks_like_url(q: &str) -> bool {
    if q.contains(char::is_whitespace) {
        return false;
    }
    if q.starts_with("http://") || q.starts_with("https://") {
        return true;
    }
    let host = q.split('/').next().unwrap_or("");
    host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host.rsplit('.').next().is_some_and(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
}

pub fn format_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{v:.10}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Evaluates `+ - * / ^` with parentheses; only used when the text looks like arithmetic.
pub fn eval_expression(src: &str) -> Option<f64> {
    let first = src.trim().chars().next()?;
    if !(first.is_ascii_digit() || matches!(first, '(' | '-' | '.')) {
        return None;
    }
    if !src.chars().any(|c| matches!(c, '+' | '-' | '*' | '/' | '^' | '(' | 'x' | '×' | '÷')) {
        return None;
    }
    let mut p = Parser { s: src.chars().filter(|c| !c.is_whitespace()).collect(), i: 0 };
    let v = p.expr()?;
    (p.i == p.s.len() && v.is_finite()).then_some(v)
}

struct Parser {
    s: Vec<char>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        while let Some(c @ ('+' | '-')) = self.peek() {
            self.i += 1;
            let r = self.term()?;
            v = if c == '+' { v + r } else { v - r };
        }
        Some(v)
    }

    fn term(&mut self) -> Option<f64> {
        let mut v = self.power()?;
        while let Some(c @ ('*' | '/' | 'x' | '×' | '÷')) = self.peek() {
            self.i += 1;
            let r = self.power()?;
            v = if matches!(c, '/' | '÷') { v / r } else { v * r };
        }
        Some(v)
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if self.peek() == Some('^') {
            self.i += 1;
            let exp = self.power()?;
            return Some(base.powf(exp));
        }
        Some(base)
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek()? {
            '-' => {
                self.i += 1;
                Some(-self.unary()?)
            }
            '+' => {
                self.i += 1;
                self.unary()
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) -> Option<f64> {
        if self.peek()? == '(' {
            self.i += 1;
            let v = self.expr()?;
            if self.peek()? != ')' {
                return None;
            }
            self.i += 1;
            return Some(v);
        }
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
            self.i += 1;
        }
        self.s[start..self.i].iter().collect::<String>().parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<App> {
        ["Firefox", "Files", "Visual Studio Code", "Spotify"]
            .iter()
            .map(|n| App { name: n.to_string(), target: format!("run-{n}"), ..Default::default() })
            .collect()
    }

    #[test]
    fn calculator() {
        assert_eq!(eval_expression("2+3*4"), Some(14.0));
        assert_eq!(eval_expression("(2+3)*4"), Some(20.0));
        assert_eq!(eval_expression("2^3^2"), Some(512.0));
        assert_eq!(eval_expression("-4+10/4"), Some(-1.5));
        assert_eq!(eval_expression("1/0"), None);
        assert_eq!(eval_expression("firefox"), None);
        assert_eq!(eval_expression("2+"), None);
        assert_eq!(format_number(2.5), "2.5");
        assert_eq!(format_number(14.0), "14");
    }

    #[test]
    fn ranks_prefix_above_subsequence() {
        let r = search("fi", &apps());
        assert_eq!(r[0].title, "Files");
        assert!(r.iter().any(|i| i.title == "Firefox"));
        assert_eq!(search("ffx", &apps())[0].title, "Firefox");
        assert_eq!(search("code", &apps())[0].title, "Visual Studio Code");
    }

    #[test]
    fn empty_query_suggests_visus_commands() {
        let r = search("", &apps());
        assert_eq!(r.len(), MAX_RESULTS);
        assert!(r.iter().all(|i| i.kind == Kind::Visus));
    }

    #[test]
    fn math_url_and_web_fallback() {
        assert_eq!(search("12*7", &apps())[0].title, "84");
        let u = search("github.com/x", &apps());
        assert_eq!(u[0].action, Some(Action::OpenUrl("https://github.com/x".into())));
        let w = search("zzzqq", &apps());
        assert_eq!(w.last().map(|i| i.kind), Some(Kind::Web));
    }

    #[test]
    fn selection_wraps() {
        let mut p = Palette::default();
        p.refresh();
        p.move_selection(-1);
        assert_eq!(p.selected, p.results.len() - 1);
    }

    #[test]
    fn note_and_window_queries() {
        let r = search_with("note buy milk", &[], &[]);
        assert!(matches!(&r[0].action, Some(Action::AddNote(t)) if t == "buy milk"));
        let wins = vec![(7isize, "Quarterly report - Word".to_string())];
        let r = search_with("quarterly", &[], &wins);
        assert!(r.iter().any(|i| i.action == Some(Action::FocusWindow(7))));
        assert!(search_with("q", &[], &wins).iter().all(|i| i.kind != Kind::Window));
    }
}
