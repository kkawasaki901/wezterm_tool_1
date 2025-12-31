use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, TimeZone};
use walkdir::WalkDir;

use crate::config::Config;
use crate::frontmatter::{parse_todo_file, render_todo_file};
use crate::model::{Status, TodoFile};

use std::io::Write;
use std::process::{Command, Stdio};

pub fn ensure_dirs(cfg: &Config) -> Result<()> {
    std::fs::create_dir_all(cfg.active_dir())?;
    std::fs::create_dir_all(cfg.done_dir())?;
    std::fs::create_dir_all(cfg.canceled_dir())?;
    std::fs::create_dir_all(cfg.templates_dir())?;
    Ok(())
}

pub fn load_active(cfg: &Config) -> Result<Vec<TodoFile>> {
    load_from_dir_recursive(&cfg.active_dir())
}

/// active + archived done/canceled (reopen対象)
pub fn load_closed(cfg: &Config) -> Result<Vec<TodoFile>> {
    let mut out = Vec::new();

    // active配下に done/canceled が残っている場合にも対応
    out.extend(load_from_dir_recursive(&cfg.active_dir())?);

    // archive済み
    out.extend(load_from_dir_recursive(&cfg.done_dir())?);
    out.extend(load_from_dir_recursive(&cfg.canceled_dir())?);

    out.retain(|t| matches!(t.fm.status, Status::Done | Status::Canceled));
    Ok(out)
}

fn load_from_dir_recursive(dir: &std::path::Path) -> Result<Vec<TodoFile>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("md") { continue; }
        let text = std::fs::read_to_string(entry.path())?;
        let todo = parse_todo_file(entry.path().to_path_buf(), &text)?;
        out.push(todo);
    }
    Ok(out)
}

/// Resolve id or prefix within ACTIVE directory.
pub fn resolve_one(cfg: &Config, id_or_prefix: &str) -> Result<TodoFile> {
    let list = load_active(cfg)?;
    resolve_from_list(&list, id_or_prefix)
}

/// Resolve id or prefix within CLOSED set (active+archive done/canceled).
pub fn resolve_one_closed(cfg: &Config, id_or_prefix: &str) -> Result<TodoFile> {
    let list = load_closed(cfg)?;
    resolve_from_list(&list, id_or_prefix)
}

fn resolve_from_list(list: &[TodoFile], id_or_prefix: &str) -> Result<TodoFile> {
    let mut matches: Vec<TodoFile> = list
        .iter()
        .cloned()
        .filter(|t| t.fm.id == id_or_prefix || t.fm.id.starts_with(id_or_prefix))
        .collect();

    if matches.is_empty() {
        return Err(anyhow!("no match for: {}", id_or_prefix));
    }
    if matches.len() == 1 {
        return Ok(matches.remove(0));
    }

    if let Some(selected) = fzf_select_todos(&matches) {
        return Ok(selected);
    }

    let mut msg = String::from("multiple matches (use longer prefix or install fzf):\n");
    for t in matches.iter().take(30) {
        msg.push_str(&format!(
            "  {}  [{}]  {}  ({})\n",
            t.short_id(),
            t.fm.importance,
            t.fm.title,
            t.fm.tags.join(",")
        ));
    }
    Err(anyhow!(msg))
}

pub fn save(todo: &TodoFile) -> Result<()> {
    let text = render_todo_file(&todo.fm, &todo.body)?;
    std::fs::write(&todo.path, text)?;
    Ok(())
}

pub fn now_jst_rfc3339() -> String {
    Local::now().to_rfc3339()
}

/// Parse due string to DateTime<FixedOffset>
/// - YYYY-MM-DD => 23:59:59 local time (JST)
/// - RFC3339 => that datetime
pub fn parse_due_dt(due: &str) -> Option<DateTime<FixedOffset>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(due) {
        return Some(dt);
    }
    if let Ok(date) = NaiveDate::parse_from_str(due, "%Y-%m-%d") {
        let local_dt = Local
            .from_local_datetime(&date.and_hms_opt(23, 59, 59)?)
            .single()?;
        return Some(local_dt.with_timezone(local_dt.offset()));
    }
    None
}

/// RFC3339 / YYYY-MM-DD をざっくり受けるパーサ（organize用）
fn parse_any_dt(s: &str) -> Option<DateTime<FixedOffset>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt);
    }
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let local_dt = Local
            .from_local_datetime(&date.and_hms_opt(23, 59, 59)?)
            .single()?;
        return Some(local_dt.with_timezone(local_dt.offset()));
    }
    None
}

pub fn month_dir(base: &std::path::Path, done_at: &str) -> Result<std::path::PathBuf> {
    let dt = DateTime::parse_from_rfc3339(done_at)
        .map_err(|_| anyhow!("invalid done_at: {}", done_at))?;
    Ok(base
        .join(format!("{:04}", dt.year()))
        .join(format!("{:02}", dt.month())))
}

pub fn move_to_archive(cfg: &Config, todo: &TodoFile) -> Result<std::path::PathBuf> {
    let done_at = todo
        .fm
        .done_at
        .as_deref()
        .ok_or_else(|| anyhow!("done_at missing"))?;

    let base = match todo.fm.status {
        Status::Done => cfg.done_dir(),
        Status::Canceled => cfg.canceled_dir(),
        _ => return Err(anyhow!("only done/canceled can be archived")),
    };

    let dest_dir = month_dir(&base, done_at)?;
    std::fs::create_dir_all(&dest_dir)?;

    let file_name = todo.path.file_name().ok_or_else(|| anyhow!("bad filename"))?;
    let dest = dest_dir.join(file_name);

    std::fs::rename(&todo.path, &dest)?;
    Ok(dest)
}

fn has_cmd(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// fzf-select from ACTIVE todos (todo/doing/waiting)
pub fn select_active_one_fzf(cfg: &Config) -> Result<Option<TodoFile>> {
    let mut list = load_active(cfg)?;
    list.retain(|t| t.fm.status.is_active());
    if list.is_empty() {
        return Ok(None);
    }
    Ok(fzf_select_todos(&list))
}

/// fzf-select from CLOSED todos (done/canceled) including archived
pub fn select_closed_one_fzf(cfg: &Config) -> Result<Option<TodoFile>> {
    let list = load_closed(cfg)?;
    if list.is_empty() {
        return Ok(None);
    }
    Ok(fzf_select_todos(&list))
}

/// Candidate format (TAB-delimited):
///   1: id (hidden)
///   2: display text (shown)
///   3: path (for preview)
/// fzf:
/// - preview: bat/batcat or sed
/// - Ctrl-O: open in $EDITOR (or nvim) without leaving fzf
fn fzf_select_todos(matches: &[TodoFile]) -> Option<TodoFile> {
    if !has_cmd("fzf") {
        return None;
    }

    let use_bat = has_cmd("bat") || has_cmd("batcat");
    let bat_cmd = if has_cmd("bat") { "bat" } else { "batcat" };

    let mut lines = String::new();
    for t in matches {
        let due = t.fm.due.clone().unwrap_or_else(|| "----".to_string());
        let tags = if t.fm.tags.is_empty() {
            "".to_string()
        } else {
            format!(" ({})", t.fm.tags.join(","))
        };
        let display = format!(
            "[{}] {} {}{}  {}",
            t.fm.importance,
            due,
            t.fm.title,
            tags,
            t.path.display()
        );
        lines.push_str(&format!(
            "{}\t{}\t{}\n",
            t.fm.id,
            display,
            t.path.display()
        ));
    }

    let preview = if use_bat {
        format!("{bat_cmd} --style=numbers --color=always --line-range :200 {{3}}")
    } else {
        "sh -lc 'sed -n \"1,200p\" \"{3}\"'".to_string()
    };

    let bind_ctrl_o = "ctrl-o:execute-silent(sh -lc '${EDITOR:-nvim} \"{3}\"')";

    let mut child = Command::new("fzf")
        .args([
            "--delimiter=\t",
            "--with-nth=2",
            "--prompt=todo> ",
            "--height=40%",
            "--reverse",
            "--preview-window=right:60%:wrap",
            "--preview",
            &preview,
            "--bind",
            bind_ctrl_o,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    {
        let stdin = child.stdin.as_mut()?;
        stdin.write_all(lines.as_bytes()).ok()?;
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let selected = String::from_utf8_lossy(&output.stdout);
    let selected_line = selected.lines().next()?.trim();
    if selected_line.is_empty() {
        return None;
    }

    let id = selected_line.split('\t').next()?.trim();
    matches.iter().find(|t| t.fm.id == id).cloned()
}

fn move_file_avoiding_collision(src: &std::path::Path, dest_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(dest_dir)?;

    let file_name = src
        .file_name()
        .ok_or_else(|| anyhow!("bad filename"))?
        .to_owned();

    let mut dest = dest_dir.join(&file_name);

    if dest.exists() {
        let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("todo");
        let ext = dest.extension().and_then(|e| e.to_str()).unwrap_or("md");
        for i in 1..=9999 {
            let cand = dest_dir.join(format!("{}_{}.{}", stem, i, ext));
            if !cand.exists() {
                dest = cand;
                break;
            }
        }
    }

    std::fs::rename(src, &dest)?;
    Ok(dest)
}

fn quarantine_broken(root: &std::path::Path, path: &std::path::Path) -> Result<std::path::PathBuf> {
    let broken_dir = root.join("broken");
    move_file_avoiding_collision(path, &broken_dir)
}

/// reopen用：active/ に戻し、さらに「新しいTS + slug」にリネーム
pub fn move_to_active(cfg: &Config, todo: &TodoFile) -> Result<std::path::PathBuf> {
    let active = cfg.active_dir();
    std::fs::create_dir_all(&active)?;

    let now_ts = Local::now().format("%Y%m%d%H%M%S").to_string();
    let slug_s = {
        let s = slug::slugify(&todo.fm.title);
        if s.is_empty() { "todo".to_string() } else { s }
    };
    let base_name = format!("{}__{}.md", now_ts, slug_s);

    let mut dest = active.join(base_name);
    if dest.exists() {
        let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("todo");
        let ext = dest.extension().and_then(|e| e.to_str()).unwrap_or("md");
        for i in 1..=9999 {
            let cand = active.join(format!("{}_{}.{}", stem, i, ext));
            if !cand.exists() {
                dest = cand;
                break;
            }
        }
    }

    if todo.path.starts_with(&active) {
        if todo.path != dest {
            std::fs::rename(&todo.path, &dest)?;
        }
        return Ok(dest);
    }

    std::fs::rename(&todo.path, &dest)?;
    Ok(dest)
}

/// archive等から active/ に戻す（ファイル名維持）＋ restored_from + 復旧ログ
pub fn restore_to_active_preserve_name(cfg: &Config, todo: &TodoFile) -> Result<std::path::PathBuf> {
    let active = cfg.active_dir();
    std::fs::create_dir_all(&active)?;

    if todo.path.starts_with(&active) {
        return Ok(todo.path.clone());
    }

    let src_str = todo.path.display().to_string();
    let dest = move_file_avoiding_collision(&todo.path, &active)?;

    if let Ok(text) = std::fs::read_to_string(&dest) {
        if let Ok(mut tf) = parse_todo_file(dest.clone(), &text) {
            let date = Local::now().format("%Y-%m-%d").to_string();
            tf.fm.updated_at = now_jst_rfc3339();
            tf.fm.restored_from = Some(src_str);
            tf.append_log_line(&date, "restored from archive");
            save(&tf)?;
        }
    }

    Ok(dest)
}

/// statusに応じて「正しい配置先」に置く（FixBroken/Archive organize用）
/// - active status => activeへ（ファイル名維持、restored_from/ログ追加）
/// - done/canceled => done/canceled の YYYY/MM（dtは done_at -> updated_at -> created_at から推定）
/// - 日付が取れない => unknown/
/// 移動後、restored_from を記録（移動元パス）
pub fn place_todo_by_status(cfg: &Config, todo: &TodoFile) -> Result<std::path::PathBuf> {
    match todo.fm.status {
        Status::Todo | Status::Doing | Status::Waiting => {
            return restore_to_active_preserve_name(cfg, todo);
        }
        Status::Done | Status::Canceled => {}
    }

    let desired_root = match todo.fm.status {
        Status::Done => cfg.done_dir(),
        Status::Canceled => cfg.canceled_dir(),
        _ => unreachable!(),
    };

    let dt_source = todo
        .fm
        .done_at
        .as_deref()
        .or(Some(todo.fm.updated_at.as_str()))
        .or(Some(todo.fm.created_at.as_str()));

    let dest_dir = if let Some(s) = dt_source {
        if let Some(dt) = parse_any_dt(s) {
            desired_root
                .join(format!("{:04}", dt.year()))
                .join(format!("{:02}", dt.month()))
        } else {
            desired_root.join("unknown")
        }
    } else {
        desired_root.join("unknown")
    };

    let src_str = todo.path.display().to_string();
    let dest = move_file_avoiding_collision(&todo.path, &dest_dir)?;

    if let Ok(text) = std::fs::read_to_string(&dest) {
        if let Ok(mut tf) = parse_todo_file(dest.clone(), &text) {
            tf.fm.updated_at = now_jst_rfc3339();
            tf.fm.restored_from = Some(src_str);
            save(&tf)?;
        }
    }

    Ok(dest)
}

/// archive 整理：done/ と canceled/ を走査し、
/// - 壊れていれば broken/
/// - active status なら active へ復旧（ログ+restored_from）
/// - done/canceled は statusに従って YYYY/MM or unknown へ
pub fn organize_archive(cfg: &Config) -> Result<usize> {
    let mut moved = 0;
    moved += organize_archive_root(cfg, &cfg.done_dir())?;
    moved += organize_archive_root(cfg, &cfg.canceled_dir())?;
    Ok(moved)
}

fn organize_archive_root(cfg: &Config, root: &std::path::Path) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }

    let mut moved = 0;

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("md") { continue; }

        let path = entry.path().to_path_buf();

        let text = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => {
                quarantine_broken(root, &path)?;
                moved += 1;
                continue;
            }
        };

        let todo = match parse_todo_file(path.clone(), &text) {
            Ok(t) => t,
            Err(_) => {
                quarantine_broken(root, &path)?;
                moved += 1;
                continue;
            }
        };

        place_todo_by_status(cfg, &todo)?;
        moved += 1;
    }

    Ok(moved)
}

/// brokenファイル選択用（TodoFileにパースできないので Path だけ）
pub fn select_broken_path_fzf(cfg: &Config) -> Result<Option<std::path::PathBuf>> {
    let mut paths = Vec::new();
    let done_broken = cfg.done_dir().join("broken");
    let canceled_broken = cfg.canceled_dir().join("broken");

    for d in [done_broken, canceled_broken] {
        if !d.exists() { continue; }
        for entry in WalkDir::new(d).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() { continue; }
            if entry.path().extension().and_then(|s| s.to_str()) != Some("md") { continue; }
            paths.push(entry.path().to_path_buf());
        }
    }

    if paths.is_empty() {
        return Ok(None);
    }

    Ok(fzf_select_paths(&paths))
}

fn fzf_select_paths(paths: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    if !has_cmd("fzf") {
        return None;
    }

    let use_bat = has_cmd("bat") || has_cmd("batcat");
    let bat_cmd = if has_cmd("bat") { "bat" } else { "batcat" };

    let mut lines = String::new();
    for p in paths {
        lines.push_str(&format!("{}\n", p.display()));
    }

    let preview = if use_bat {
        format!("{bat_cmd} --style=numbers --color=always --line-range :200 {{}}")
    } else {
        "sh -lc 'sed -n \"1,200p\" \"{}\"'".to_string()
    };

    let bind_ctrl_o = "ctrl-o:execute-silent(sh -lc '${EDITOR:-nvim} \"{}\"')";

    let mut child = Command::new("fzf")
        .args([
            "--prompt=broken> ",
            "--height=40%",
            "--reverse",
            "--preview-window=right:60%:wrap",
            "--preview",
            &preview,
            "--bind",
            bind_ctrl_o,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    {
        let stdin = child.stdin.as_mut()?;
        stdin.write_all(lines.as_bytes()).ok()?;
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let selected = String::from_utf8_lossy(&output.stdout);
    let sel = selected.lines().next()?.trim();
    if sel.is_empty() {
        return None;
    }

    Some(std::path::PathBuf::from(sel))
}
