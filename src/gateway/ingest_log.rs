use crate::envelope::StampedEnvelopeV1;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Backward-compatible append to a fixed path (no rotation)
pub fn append(path: &Path, stamped: &StampedEnvelopeV1) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(stamped)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

/// Append to a daily-rotated ingest log file under `log_dir`.
/// Pattern: ingest_YYYY-MM-DD.ndjson and a symlink `ingest.ndjson` pointing to current.
pub fn append_rotating(log_dir: &Path, stamped: &StampedEnvelopeV1) -> anyhow::Result<()> {
    // Ensure directory exists
    fs::create_dir_all(log_dir)?;

    // Compute today's file name
    let date_str = Utc::now().format("%Y-%m-%d");
    let file_name = format!("ingest_{}.ndjson", date_str);
    let target_path = log_dir.join(&file_name);

    // Ensure symlink points to today's file
    let symlink_path = log_dir.join("ingest.ndjson");
    ensure_symlink_to_current(&symlink_path, &target_path)?;

    // Append to the target file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target_path)?;
    let line = serde_json::to_string(stamped)?;
    match writeln!(file, "{}", line) {
        Ok(_) => {
            crate::metrics::ingest_log::write_success();
            crate::metrics::ingest_log::write_bytes(line.len());
        }
        Err(e) => {
            crate::metrics::ingest_log::write_error();
            return Err(e.into());
        }
    }

    // Update current file size
    if let Ok(metadata) = file.metadata() {
        crate::metrics::ingest_log::current_file_bytes(metadata.len());
    }

    Ok(())
}

fn ensure_symlink_to_current(link_path: &Path, target_path: &Path) -> anyhow::Result<()> {
    // If link exists, check if it already points to target; otherwise, replace it.
    if link_path.exists() {
        // Try to read current link; if not a symlink, remove it
        let mut needs_update = true;
        if let Ok(curr_target) = fs::read_link(link_path) {
            // If the current target matches, no update
            if paths_equivalent(&curr_target, target_path) {
                needs_update = false;
            }
        }
        if needs_update {
            let _ = fs::remove_file(link_path);
        } else {
            return Ok(());
        }
    }
    // Create a relative symlink if possible for portability
    // Use absolute target to avoid dependency on path diffing
    #[cfg(unix)]
    std::os::unix::fs::symlink(target_path, link_path)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target_path, link_path)?;
    Ok(())
}

fn paths_equivalent(a: &Path, b: &Path) -> bool {
    // Best-effort comparison using canonicalize; fall back to direct compare
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ac), Ok(bc)) => ac == bc,
        _ => a == b,
    }
}
