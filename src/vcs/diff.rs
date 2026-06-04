use anyhow::{Result, anyhow};
use similar::{ChangeTag, TextDiff};

pub fn unified_diff(old: &str, new: &str, old_name: &str, new_name: &str) -> Result<String> {
    let diff = TextDiff::from_lines(old, new);
    let mut out = format!("# TranslateR TPatch v1\n--- {old_name}\n+++ {new_name}\n");
    for group in diff.grouped_ops(3) {
        for op in group {
            for change in diff.iter_changes(&op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                out.push_str(prefix);
                out.push_str(change.value());
            }
        }
    }
    Ok(out)
}

pub fn apply_unified_patch(original: &str, patch: &str) -> Result<String> {
    let mut out = Vec::<String>::new();
    let original_lines = original
        .split_inclusive('\n')
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let patch_lines = patch.split_inclusive('\n').collect::<Vec<_>>();
    let mut original_idx = 0usize;
    let mut patch_idx = 0usize;

    while patch_idx < patch_lines.len() {
        let line = patch_lines[patch_idx];
        if line.starts_with("# TranslateR TPatch ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
        {
            patch_idx += 1;
            continue;
        }
        if line.starts_with("@@") {
            patch_idx += 1;
            continue;
        }
        let Some(prefix) = line.chars().next() else {
            patch_idx += 1;
            continue;
        };
        let content = &line[1..];
        match prefix {
            ' ' => {
                let Some(found_idx) = find_line(&original_lines, original_idx, content) else {
                    return Err(anyhow!("patch context did not match active PO file"));
                };
                out.extend(original_lines[original_idx..found_idx].iter().cloned());
                out.push(content.to_string());
                original_idx = found_idx + 1;
            }
            '-' => {
                let Some(found_idx) = find_line(&original_lines, original_idx, content) else {
                    return Err(anyhow!("patch deletion did not match active PO file"));
                };
                out.extend(original_lines[original_idx..found_idx].iter().cloned());
                original_idx = found_idx + 1;
            }
            '+' => out.push(content.to_string()),
            _ => return Err(anyhow!("unsupported patch line: {}", line.trim_end())),
        }
        patch_idx += 1;
    }

    out.extend(original_lines.into_iter().skip(original_idx));
    Ok(out.concat())
}

fn find_line(lines: &[String], start: usize, target: &str) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(idx, line)| (line.as_str() == target).then_some(idx))
}
