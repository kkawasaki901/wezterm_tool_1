mod cli;
mod config;
mod model;
mod frontmatter;
mod store;
mod format;

use anyhow::{anyhow, Result};
use clap::Parser;
use chrono::Local;
use std::collections::HashSet;
use std::process::Command;

use cli::{Args, Cmd};
use config::Config;
use model::{FrontMatter, Status, TodoFile};

fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = Config::load();
    store::ensure_dirs(&cfg)?;

    match args.cmd {
        Cmd::Add { title, due, tags, importance, edit, slug } => {
            cmd_add(&cfg, title, due, tags, importance, edit, slug)
        }
        Cmd::List { due_within, due_from, due_to, tag, status, importance, text, include_overdue } => {
            cmd_list(&cfg, due_within, due_from, due_to, tag, status, importance, text, include_overdue)
        }
        Cmd::Show { id_or_prefix } => cmd_show(&cfg, &id_or_prefix),
        Cmd::Edit { id_or_prefix } => cmd_edit(&cfg, &id_or_prefix),

        Cmd::Start { id_or_prefix } => cmd_start(&cfg, id_or_prefix),
        Cmd::Wait { id_or_prefix } => cmd_wait(&cfg, id_or_prefix),
        Cmd::Done { id_or_prefix } => cmd_done(&cfg, id_or_prefix),
        Cmd::Cancel { id_or_prefix } => cmd_cancel(&cfg, id_or_prefix),

        Cmd::Reopen { id_or_prefix } => cmd_reopen(&cfg, id_or_prefix),

        Cmd::Archive => cmd_archive(&cfg),

        Cmd::FixBroken => cmd_fix_broken(&cfg),
    }
}

fn cmd_add(
    cfg: &Config,
    title: Option<String>,
    due: Option<String>,
    tags: Vec<String>,
    importance: i32,
    edit: bool,
    slug_opt: Option<String>,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    let id = now.clone();

    let file_ts = Local::now().format("%Y%m%d%H%M%S").to_string();
    let slug = slug_opt
        .or_else(|| title.as_ref().map(|t| slug::slugify(t)))
        .filter(|s| !s.is_empty());

    let filename = if let Some(slug) = slug {
        format!("{}__{}.md", file_ts, slug)
    } else {
        format!("{}.md", file_ts)
    };

    let path = cfg.active_dir().join(filename);

    let mut body = String::from("\n## メモ\n\n## サブタスク\n- [ ] \n\n## ログ\n- :\n");

    if cfg.template_path().exists() {
        let tpl = std::fs::read_to_string(cfg.template_path())?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        let t = tpl
            .replace("{{id}}", &id)
            .replace("{{title}}", title.as_deref().unwrap_or(""))
            .replace("{{now}}", &now)
            .replace("{{date}}", &date);

        let parsed = frontmatter::parse_todo_file(path.clone(), &t)?;
        body = parsed.body;
    }

    let fm = FrontMatter {
        id,
        title: title.unwrap_or_default(),
        status: Status::Todo,
        due,
        tags,
        importance,
        created_at: now.clone(),
        updated_at: now,
        done_at: None,
        restored_from: None,
    };

    let todo = TodoFile { path: path.clone(), fm, body };
    store::save(&todo)?;

    if edit {
        open_in_editor(cfg, &path)?;
        cmd_touch_updated_at(&path)?;
    } else {
        println!("created: {}", path.display());
    }
    Ok(())
}

fn cmd_list(
    cfg: &Config,
    due_within: Option<String>,
    due_from: Option<String>,
    due_to: Option<String>,
    tag: Option<String>,
    status: Option<String>,
    importance_expr: Option<String>,
    text: Option<String>,
    include_overdue: bool,
) -> Result<()> {
    let mut todos = store::load_active(cfg)?;
    todos.retain(|t| t.fm.status.is_active());

    if let Some(s) = status {
        let want: Status = s.parse()?;
        todos.retain(|t| t.fm.status == want);
    }

    if let Some(tag) = tag {
        let tag = tag.to_lowercase();
        todos.retain(|t| t.fm.tags.iter().any(|x| x.to_lowercase() == tag));
    }

    if let Some(expr) = importance_expr {
        let (op, n) = parse_importance_expr(&expr)?;
        todos.retain(|t| compare_i32(t.fm.importance, op, n));
    }

    if let Some(q) = text {
        if let Some(paths) = rg_paths(&cfg.active_dir(), &q) {
            todos.retain(|t| paths.contains(&t.path));
        } else {
            let q = q.to_lowercase();
            todos.retain(|t| t.fm.title.to_lowercase().contains(&q) || t.body.to_lowercase().contains(&q));
        }
    }

    let now_fixed = Local::now().with_timezone(Local::now().offset());

    if let Some(within) = due_within {
        let days = parse_days(&within)?;
        let end = now_fixed + chrono::Duration::days(days);
        todos.retain(|t| {
            let Some(due_str) = t.fm.due.as_deref() else { return false; };
            let Some(due_dt) = store::parse_due_dt(due_str) else { return false; };
            if !include_overdue && due_dt < now_fixed { return false; }
            due_dt >= now_fixed && due_dt <= end
        });
    } else if due_from.is_some() || due_to.is_some() {
        let from_dt = due_from.as_deref().and_then(store::parse_due_dt);
        let to_dt = due_to.as_deref().and_then(store::parse_due_dt);
        todos.retain(|t| {
            let Some(due_str) = t.fm.due.as_deref() else { return false; };
            let Some(due_dt) = store::parse_due_dt(due_str) else { return false; };
            if let Some(f) = from_dt { if due_dt < f { return false; } }
            if let Some(to) = to_dt { if due_dt > to { return false; } }
            true
        });
    }

    todos.sort_by(|a, b| {
        use std::cmp::Ordering;

        let a_due = a.fm.due.as_deref().and_then(store::parse_due_dt);
        let b_due = b.fm.due.as_deref().and_then(store::parse_due_dt);

        let a_over = a_due.map(|d| d < now_fixed).unwrap_or(false);
        let b_over = b_due.map(|d| d < now_fixed).unwrap_or(false);

        match (a_over, b_over) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        match (a_due, b_due) {
            (Some(ad), Some(bd)) => if ad != bd { return ad.cmp(&bd); },
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (None, None) => {}
        }

        if a.fm.importance != b.fm.importance {
            return b.fm.importance.cmp(&a.fm.importance);
        }

        a.fm.id.cmp(&b.fm.id)
    });

    let enable_color = std::env::var("NO_COLOR").is_err();

    for t in &todos {
        let l = format::label_for(t, cfg.soon_days);
        let lab = format::label_colored(l, enable_color);
        let due = format::due_display(t);
        let imp = format!("[{}]", t.fm.importance);
        let title = format::truncate(&t.fm.title, 40);
        let tags = if t.fm.tags.is_empty() { "".to_string() } else { format!(" ({})", t.fm.tags.join(",")) };

        println!(
            "{:<7} {:<10} {:<6} {:<12} {:<40}{}",
            lab, due, imp, t.short_id(), title, tags
        );
    }

    Ok(())
}

fn cmd_show(cfg: &Config, id_or_prefix: &str) -> Result<()> {
    let todo = store::resolve_one(cfg, id_or_prefix)?;
    let text = std::fs::read_to_string(&todo.path)?;
    println!("{}", text);
    Ok(())
}

fn cmd_edit(cfg: &Config, id_or_prefix: &str) -> Result<()> {
    let todo = store::resolve_one(cfg, id_or_prefix)?;
    let path = todo.path.clone();

    open_in_editor(cfg, &path)?;
    cmd_touch_updated_at(&path)?;

    println!("updated: {}", path.display());
    Ok(())
}

fn cmd_touch_updated_at(path: &std::path::Path) -> Result<()> {
    let text = std::fs::read_to_string(path)?;
    let mut edited = frontmatter::parse_todo_file(path.to_path_buf(), &text)?;
    edited.fm.updated_at = store::now_jst_rfc3339();

    if matches!(edited.fm.status, Status::Done | Status::Canceled) && edited.fm.done_at.is_none() {
        edited.fm.done_at = Some(store::now_jst_rfc3339());
    }

    store::save(&edited)?;
    Ok(())
}

fn pick_or_prefix(
    cfg: &Config,
    id_or_prefix: Option<String>,
    selector: fn(&Config) -> Result<Option<TodoFile>>,
) -> Result<Option<String>> {
    if let Some(s) = id_or_prefix {
        return Ok(Some(s));
    }
    if let Some(todo) = selector(cfg)? {
        return Ok(Some(todo.fm.id));
    }
    Ok(None)
}

fn cmd_start(cfg: &Config, id_or_prefix: Option<String>) -> Result<()> {
    if let Some(id) = pick_or_prefix(cfg, id_or_prefix, store::select_active_one_fzf)? {
        return cmd_set_status(cfg, &id, Status::Doing);
    }
    println!("no selection (fzf not available / canceled / no candidates)");
    Ok(())
}

fn cmd_wait(cfg: &Config, id_or_prefix: Option<String>) -> Result<()> {
    if let Some(id) = pick_or_prefix(cfg, id_or_prefix, store::select_active_one_fzf)? {
        return cmd_set_status(cfg, &id, Status::Waiting);
    }
    println!("no selection (fzf not available / canceled / no candidates)");
    Ok(())
}

fn cmd_done(cfg: &Config, id_or_prefix: Option<String>) -> Result<()> {
    if let Some(id) = pick_or_prefix(cfg, id_or_prefix, store::select_active_one_fzf)? {
        return cmd_set_status(cfg, &id, Status::Done);
    }
    println!("no selection (fzf not available / canceled / no candidates)");
    Ok(())
}

fn cmd_cancel(cfg: &Config, id_or_prefix: Option<String>) -> Result<()> {
    if let Some(id) = pick_or_prefix(cfg, id_or_prefix, store::select_active_one_fzf)? {
        return cmd_set_status(cfg, &id, Status::Canceled);
    }
    println!("no selection (fzf not available / canceled / no candidates)");
    Ok(())
}

/// reopen:
/// - 引数あり/なしで closed(done/canceled)から取得（archive含む）
/// - active/ に戻してリネーム（TS+slug）
/// - restored_from を記録
fn cmd_reopen(cfg: &Config, id_or_prefix: Option<String>) -> Result<()> {
    let mut todo = if let Some(s) = id_or_prefix {
        store::resolve_one_closed(cfg, &s)?
    } else {
        match store::select_closed_one_fzf(cfg)? {
            Some(t) => t,
            None => {
                println!("no selection (fzf not available / canceled / no closed todos)");
                return Ok(());
            }
        }
    };

    if !matches!(todo.fm.status, Status::Done | Status::Canceled) {
        anyhow::bail!(
            "reopen is only allowed for status done/canceled, but got: {}",
            todo.fm.status.as_str()
        );
    }

    let src_str = todo.path.display().to_string();

    let new_path = store::move_to_active(cfg, &todo)?;
    todo.path = new_path;

    let now = store::now_jst_rfc3339();
    let date = Local::now().format("%Y-%m-%d").to_string();

    let prev = todo.fm.status.clone();
    let next = Status::Todo;

    todo.fm.status = next.clone();
    todo.fm.updated_at = now;
    todo.fm.done_at = None;
    todo.fm.restored_from = Some(src_str);

    let msg = format!("reopen (status {} -> {})", prev.as_str(), next.as_str());
    todo.append_log_line(&date, &msg);

    store::save(&todo)?;
    println!("reopened: {}", todo.path.display());
    Ok(())
}

fn cmd_set_status(cfg: &Config, id_or_prefix: &str, status: Status) -> Result<()> {
    let mut todo = store::resolve_one(cfg, id_or_prefix)?;
    let now = store::now_jst_rfc3339();
    let date = Local::now().format("%Y-%m-%d").to_string();

    let prev = todo.fm.status.clone();

    todo.fm.status = status.clone();
    todo.fm.updated_at = now.clone();

    match status {
        Status::Done | Status::Canceled => todo.fm.done_at = Some(now.clone()),
        Status::Todo | Status::Doing | Status::Waiting => todo.fm.done_at = None,
    }

    let action = match (&prev, &status) {
        (Status::Todo, Status::Doing) => "start",
        (_, Status::Doing) => "set doing",
        (_, Status::Waiting) => "set waiting",
        (_, Status::Done) => "done",
        (_, Status::Canceled) => "canceled",
        (_, Status::Todo) => "reopen",
    };

    let msg = format!("{} (status {} -> {})", action, prev.as_str(), status.as_str());
    todo.append_log_line(&date, &msg);

    store::save(&todo)?;

    if cfg.auto_archive && matches!(todo.fm.status, Status::Done | Status::Canceled) {
        let dest = store::move_to_archive(cfg, &todo)?;
        println!("archived: {}", dest.display());
    } else {
        println!("updated: {}", todo.path.display());
    }

    Ok(())
}

fn cmd_archive(cfg: &Config) -> Result<()> {
    // 1) active/ の done/canceled を archive へ
    let todos = store::load_active(cfg)?;
    let mut moved_from_active = 0;

    for t in todos {
        if matches!(t.fm.status, Status::Done | Status::Canceled) && t.fm.done_at.is_some() {
            store::move_to_archive(cfg, &t)?;
            moved_from_active += 1;
        }
    }

    // 2) archive 内を整理（statusズレ修正、active復旧、broken隔離、YYYY/MM整形）
    let reorganized = store::organize_archive(cfg)?;

    println!(
        "archived {} file(s) from active, reorganized {} file(s) in archive",
        moved_from_active, reorganized
    );
    Ok(())
}

/// done/broken と canceled/broken から選んで修復
fn cmd_fix_broken(cfg: &Config) -> Result<()> {
    let Some(path) = store::select_broken_path_fzf(cfg)? else {
        println!("no broken files (or fzf not available / canceled)");
        return Ok(());
    };

    open_in_editor(cfg, &path)?;

    let text = std::fs::read_to_string(&path)?;
    let todo = match frontmatter::parse_todo_file(path.clone(), &text) {
        Ok(t) => t,
        Err(e) => {
            println!("still broken: {} ({})", path.display(), e);
            return Ok(());
        }
    };

    let dest = store::place_todo_by_status(cfg, &todo)?;
    println!("fixed and placed: {}", dest.display());
    Ok(())
}

fn open_in_editor(cfg: &Config, path: &std::path::Path) -> Result<()> {
    let editor = &cfg.editor;
    let status = Command::new(editor)
        .arg(path)
        .status()
        .map_err(|e| anyhow!("failed to launch editor '{}': {}", editor, e))?;
    if !status.success() {
        return Err(anyhow!("editor exited with non-zero status"));
    }
    Ok(())
}

fn parse_days(s: &str) -> Result<i64> {
    let s = s.trim().to_lowercase();
    if let Some(num) = s.strip_suffix('d') {
        return Ok(num.parse::<i64>()?);
    }
    Err(anyhow!("invalid duration: {} (use like 14d)", s))
}

#[derive(Debug, Clone, Copy)]
enum Op { Eq, Gt, Ge, Lt, Le }

fn parse_importance_expr(s: &str) -> Result<(Op, i32)> {
    let s = s.trim();
    for (p, op) in [
        (">=", Op::Ge),
        ("<=", Op::Le),
        (">", Op::Gt),
        ("<", Op::Lt),
        ("=", Op::Eq),
    ] {
        if let Some(rest) = s.strip_prefix(p) {
            return Ok((op, rest.trim().parse()?));
        }
    }
    Ok((Op::Eq, s.parse()?))
}

fn compare_i32(v: i32, op: Op, n: i32) -> bool {
    match op {
        Op::Eq => v == n,
        Op::Gt => v > n,
        Op::Ge => v >= n,
        Op::Lt => v < n,
        Op::Le => v <= n,
    }
}

fn rg_paths(dir: &std::path::Path, query: &str) -> Option<HashSet<std::path::PathBuf>> {
    let out = Command::new("rg")
        .arg("-l")
        .arg(query)
        .arg(dir)
        .output()
        .ok()?;

    if !out.status.success() { return None; }

    let s = String::from_utf8_lossy(&out.stdout);
    let mut set = HashSet::new();
    for line in s.lines() {
        let p = line.trim();
        if !p.is_empty() {
            set.insert(std::path::PathBuf::from(p));
        }
    }
    Some(set)
}
