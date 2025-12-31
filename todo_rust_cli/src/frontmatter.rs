use anyhow::{anyhow, Result};
use crate::model::{FrontMatter, TodoFile};

pub fn parse_todo_file(path: std::path::PathBuf, text: &str) -> Result<TodoFile> {
    let (fm_str, body) = split_frontmatter(text)?;
    let fm: FrontMatter = serde_yaml::from_str(&fm_str)
        .map_err(|e| anyhow!("YAML parse error in {}: {}", path.display(), e))?;
    Ok(TodoFile { path, fm, body })
}

pub fn render_todo_file(fm: &FrontMatter, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(fm)?;
    Ok(format!("---\n{}---\n{}", yaml, body.trim_start_matches('\n')))
}

fn split_frontmatter(text: &str) -> Result<(String, String)> {
    let mut lines = text.lines();

    if lines.next() != Some("---") {
        return Err(anyhow!("missing frontmatter start '---'"));
    }

    let mut yaml_lines = Vec::new();
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
        yaml_lines.push(line);
    }

    let yaml = yaml_lines.join("\n");
    let body = lines.collect::<Vec<_>>().join("\n");

    Ok((yaml, body))
}
