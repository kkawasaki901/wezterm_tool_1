use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "todo",
    version,
    about = "TODO CLI (1 file = 1 todo, Markdown + YAML frontmatter)",
    long_about = "A simple TODO manager where each TODO is stored as a Markdown file with YAML frontmatter.\n\
                  Default root: ~/todo\n\
                  Directories: active/, done/YYYY/MM/, canceled/YYYY/MM/, templates/\n\
                  Tip: done/start/wait/cancel/reopen support fzf selection when no argument is given.\n\
                  Tip: In fzf, Ctrl-O opens the selected file in $EDITOR (if available).\n\
                  Tip: todo fix-broken helps repair files quarantined in done/broken or canceled/broken."
)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Create new todo
    Add {
        /// Title (optional if --edit)
        title: Option<String>,

        /// Due date: YYYY-MM-DD or RFC3339 datetime (e.g. 2026-01-10 or 2026-01-10T18:00+09:00)
        #[arg(long)]
        due: Option<String>,

        /// Tags (comma-separated): --tags work,mail
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Importance 1..5 (default: 3)
        #[arg(long, default_value_t = 3)]
        importance: i32,

        /// Open in $EDITOR after creation
        #[arg(long)]
        edit: bool,

        /// Custom slug for filename
        #[arg(long)]
        slug: Option<String>,
    },

    /// List todos (default: active only, status in todo|doing|waiting)
    List {
        /// Due within: e.g. 14d
        #[arg(long)]
        due_within: Option<String>,

        /// Due from (inclusive): YYYY-MM-DD or RFC3339
        #[arg(long)]
        due_from: Option<String>,

        /// Due to (inclusive): YYYY-MM-DD or RFC3339
        #[arg(long)]
        due_to: Option<String>,

        /// Filter by a single tag
        #[arg(long)]
        tag: Option<String>,

        /// Filter by status: todo|doing|waiting|done|canceled
        #[arg(long)]
        status: Option<String>,

        /// Importance filter: e.g. >=4, <3, =5, or 3
        #[arg(long)]
        importance: Option<String>,

        /// Text query (uses rg if available, else fallback search)
        #[arg(long)]
        text: Option<String>,

        /// Include overdue items when using --due-within
        #[arg(long)]
        include_overdue: bool,
    },

    /// Show a todo file (id or id prefix). If multiple matches, fzf will be used if available.
    Show { id_or_prefix: String },

    /// Edit a todo file in $EDITOR and update updated_at automatically.
    /// If multiple matches, fzf will be used if available.
    Edit { id_or_prefix: String },

    /// Mark as doing (status=doing). If no argument is provided, fzf-select from ACTIVE todos.
    Start { id_or_prefix: Option<String> },

    /// Mark as waiting (status=waiting). If no argument is provided, fzf-select from ACTIVE todos.
    Wait { id_or_prefix: Option<String> },

    /// Mark as done (status=done, done_at=now). If no argument is provided, fzf-select from ACTIVE todos.
    Done { id_or_prefix: Option<String> },

    /// Cancel (status=canceled, done_at=now). If no argument is provided, fzf-select from ACTIVE todos.
    Cancel { id_or_prefix: Option<String> },

    /// Reopen (status=todo, done_at cleared).
    /// - No argument: fzf-select from CLOSED (done/canceled) todos (includes archived).
    /// - With argument: must be done/canceled or it will be rejected (includes archived).
    Reopen { id_or_prefix: Option<String> },

    /// Move done/canceled files from active/ to done/YYYY/MM or canceled/YYYY/MM
    /// and also reorganize archive (including restoring active-status files, quarantining broken files)
    Archive,

    /// Fix quarantined files in done/broken or canceled/broken
    /// - Choose a file via fzf (with preview)
    /// - Open in $EDITOR
    /// - If it becomes valid, auto-place it into active/ or done/canceled YYYY/MM
    FixBroken,
}
