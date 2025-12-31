use chrono::{DateTime, Duration, FixedOffset, Local};
use crate::model::TodoFile;
use crate::store::parse_due_dt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    Overdue,
    Today,
    Soon,
    None,
    NoDue,
}

pub fn label_for(todo: &TodoFile, soon_days: i64) -> Label {
    let now = Local::now();
    let now_fixed: DateTime<FixedOffset> = now.with_timezone(now.offset());

    let Some(due_str) = todo.fm.due.as_deref() else { return Label::NoDue; };
    let Some(due_dt) = parse_due_dt(due_str) else { return Label::None; };

    if due_dt < now_fixed { return Label::Overdue; }

    let due_local = due_dt.with_timezone(now.offset());
    if due_local.date_naive() == now.date_naive() { return Label::Today; }

    let soon_limit = now_fixed + Duration::days(soon_days);
    if due_dt <= soon_limit { return Label::Soon; }

    Label::None
}

pub fn label_str(l: Label) -> &'static str {
    match l {
        Label::Overdue => "OVERDUE",
        Label::Today => "TODAY",
        Label::Soon => "SOON",
        Label::NoDue => "NO DUE",
        Label::None => "",
    }
}

fn color(s: &str, code: &str, enable: bool) -> String {
    if !enable { return s.to_string(); }
    format!("\x1b[{}m{}\x1b[0m", code, s)
}

pub fn label_colored(l: Label, enable: bool) -> String {
    let s = label_str(l);
    match l {
        Label::Overdue => color(s, "31;1", enable),
        Label::Today => color(s, "33;1", enable),
        Label::Soon => color(s, "36;1", enable),
        Label::NoDue => color(s, "90", enable),
        Label::None => s.to_string(),
    }
}

pub fn due_display(todo: &TodoFile) -> String {
    match todo.fm.due.as_deref() {
        None => "----".to_string(),
        Some(s) => {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return d.format("%Y-%m-%d").to_string();
            }
            if let Some(dt) = parse_due_dt(s) {
                return dt.date_naive().format("%Y-%m-%d").to_string();
            }
            s.to_string()
        }
    }
}

pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars { return s.to_string(); }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}
