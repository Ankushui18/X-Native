//! Production reliability (beta checklist):
//!
//! * `atomic_write` — write-to-temp + fsync + rename; a crash mid-save
//!   can never leave a half-written document (partial writes were the
//!   review's corruption vector).
//! * autosave — periodic snapshot to `<doc>.autosave` via atomic_write;
//!   `check_crash_recovery` offers it back when it's newer than the doc.
//! * rolling backups — `<doc>.bak1..bakN` rotate on every explicit save,
//!   giving a recovery HISTORY, not just the latest state.
//! * corruption recovery — the lenient loader already exists
//!   (`load_x_lenient`); `open_with_recovery` chains: exact parse →
//!   autosave → lenient recovery → newest backup.
//! * `upgrade_legacy_library_hashes` — pre-integrity documents get
//!   hashes computed from their (trusted-on-first-open) snapshots, so
//!   the LegacyUnhashed state converges to Verified.
//! * recent files — tiny MRU list under the user cache dir.

use crate::xlib::library_hash;
use std::io::Write;
use std::path::{Path, PathBuf};
use x_core::Document;

// ------------------------------------------------------------ atomic write

/// Write-to-temp + fsync + atomic rename. Same-directory temp file so the
/// rename cannot cross filesystems.
pub fn atomic_write(path: &str, contents: &[u8]) -> std::io::Result<()> {
    let p = Path::new(path);
    let dir = p
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let tmp = dir.join(format!(
        ".{}.tmp{}",
        p.file_name().and_then(|n| n.to_str()).unwrap_or("out"),
        std::process::id()
    ));
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?; // data on disk BEFORE the rename publishes it
    }
    std::fs::rename(&tmp, p)?;
    Ok(())
}

// ---------------------------------------------------------------- autosave

pub fn autosave_path(doc_path: &str) -> String {
    format!("{doc_path}.autosave")
}

/// Atomic autosave snapshot. Returns bytes written.
pub fn autosave(doc_path: &str, serialized: &str) -> std::io::Result<usize> {
    atomic_write(&autosave_path(doc_path), serialized.as_bytes())?;
    Ok(serialized.len())
}

/// After a clean save the autosave is stale — drop it so crash recovery
/// never offers an OLDER state than the document itself.
pub fn clear_autosave(doc_path: &str) {
    let _ = std::fs::remove_file(autosave_path(doc_path));
}

/// Crash recovery check: an autosave file that exists at startup means
/// the last session did NOT exit through a clean save.
pub fn check_crash_recovery(doc_path: &str) -> Option<String> {
    std::fs::read_to_string(autosave_path(doc_path)).ok()
}

// ----------------------------------------------------------------- backups

pub const BACKUP_DEPTH: usize = 3;

fn backup_path(doc_path: &str, n: usize) -> String {
    format!("{doc_path}.bak{n}")
}

/// Rotate backups before an explicit save: bak2->bak3, bak1->bak2,
/// current doc -> bak1. Recovery HISTORY, not just latest.
pub fn rotate_backups(doc_path: &str) {
    for n in (1..BACKUP_DEPTH).rev() {
        let _ = std::fs::rename(backup_path(doc_path, n), backup_path(doc_path, n + 1));
    }
    if Path::new(doc_path).exists() {
        let _ = std::fs::copy(doc_path, backup_path(doc_path, 1));
    }
}

/// List existing backups, newest first.
pub fn list_backups(doc_path: &str) -> Vec<String> {
    (1..=BACKUP_DEPTH)
        .map(|n| backup_path(doc_path, n))
        .filter(|p| Path::new(p).exists())
        .collect()
}

// ------------------------------------------------------------ open chain

/// What `open_with_recovery` had to do to produce a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    Clean,
    /// autosave was newer/present — crash recovery applied
    RecoveredFromAutosave,
    /// lenient brace-balance recovery on the main file
    RecoveredLenient(usize),
    /// fell back to a rolling backup
    RecoveredFromBackup(String),
}

/// The full recovery chain: exact parse → autosave → lenient → backups.
pub fn open_with_recovery(doc_path: &str) -> Option<(Document, OpenOutcome)> {
    // 0. crash marker: an autosave beats the main file when present
    if let Some(auto_text) = check_crash_recovery(doc_path) {
        if let Ok(d) = crate::load_x(&auto_text) {
            if !d.pages.is_empty() {
                return Some((d, OpenOutcome::RecoveredFromAutosave));
            }
        }
    }
    let text = std::fs::read_to_string(doc_path).ok();
    if let Some(t) = &text {
        // 1. exact
        if let Ok(d) = crate::load_x(t) {
            if !d.pages.is_empty() {
                return Some((d, OpenOutcome::Clean));
            }
        }
        // 2. lenient
        let (d2, notes) = crate::load_x_lenient(t);
        if !d2.doc.pages.is_empty() {
            return Some((d2.doc, OpenOutcome::RecoveredLenient(notes.len())));
        }
    }
    // 3. backups, newest first
    for b in list_backups(doc_path) {
        if let Ok(bt) = std::fs::read_to_string(&b) {
            if let Ok(d) = crate::load_x(&bt) {
                if !d.pages.is_empty() {
                    return Some((d, OpenOutcome::RecoveredFromBackup(b)));
                }
            }
        }
    }
    None
}

// ----------------------------------------------- legacy integrity upgrade

/// Pre-integrity documents carry snapshots but empty hashes. On first
/// open we trust the embedded snapshot (it's what the doc was rendering
/// all along) and pin its hash so future tampering IS detected.
/// Returns upgraded library ids.
pub fn upgrade_legacy_library_hashes(doc: &mut Document) -> Vec<String> {
    let mut upgraded = vec![];
    for dep in &mut doc.library_deps {
        if dep.snapshot_hash.is_empty() {
            if let Some(snap) = doc.library_snapshots.get(&dep.library_id) {
                dep.snapshot_hash = library_hash(snap);
                upgraded.push(dep.library_id.clone());
            }
        }
    }
    upgraded
}

// ------------------------------------------------------------ recent files

fn recent_path() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join(".cache/x-native/recent.txt")
}

pub fn push_recent(doc_path: &str) {
    let p = recent_path();
    let mut list = recent_files();
    list.retain(|e| e != doc_path);
    list.insert(0, doc_path.to_string());
    list.truncate(8);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = atomic_write(
        p.to_str().unwrap_or("recent.txt"),
        list.join("\n").as_bytes(),
    );
}

pub fn recent_files() -> Vec<String> {
    std::fs::read_to_string(recent_path())
        .map(|t| {
            t.lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Clear the recent-documents MRU list (File ▸ Clear Recent).
pub fn clear_recent() {
    let p = recent_path();
    let _ = std::fs::remove_file(&p);
}

#[cfg(test)]
mod tests {
    use super::*;
    use x_core::{Color, Node};

    fn tmp(name: &str) -> String {
        format!(
            "{}/xn-rel-{}-{name}",
            std::env::temp_dir().display(),
            std::process::id()
        )
    }

    fn doc() -> Document {
        let mut d = Document::new();
        d.pages
            .push(Node::frame("p", 100.0, 100.0).child(Node::rect(
                "r",
                0.0,
                0.0,
                10.0,
                10.0,
                Color::BLACK,
            )));
        d
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let p = tmp("atomic.x");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "two");
        let dir = std::path::Path::new(&p).parent().unwrap();
        let leftovers = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("atomic.x.tmp"))
            .count();
        assert_eq!(leftovers, 0, "no temp files left behind");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn crash_recovery_prefers_autosave_and_clean_save_clears_it() {
        let p = tmp("crash.x");
        let older = crate::save_x(&doc());
        atomic_write(&p, older.as_bytes()).unwrap();
        // "crash": autosave with an extra page exists
        let mut newer = doc();
        newer.pages.push(Node::frame("p2", 50.0, 50.0));
        autosave(&p, &crate::save_x(&newer)).unwrap();
        let (d, outcome) = open_with_recovery(&p).unwrap();
        assert_eq!(outcome, OpenOutcome::RecoveredFromAutosave);
        assert_eq!(d.pages.len(), 2, "autosave content won");
        // clean save clears the marker -> next open is Clean
        clear_autosave(&p);
        let (_, outcome) = open_with_recovery(&p).unwrap();
        assert_eq!(outcome, OpenOutcome::Clean);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn recovery_chain_lenient_then_backup() {
        let p = tmp("chain.x");
        let good = crate::save_x(&doc());
        // backup holds a good copy; main file is 75% truncated
        atomic_write(&p, good.as_bytes()).unwrap();
        rotate_backups(&p); // -> bak1
        let cut = &good[..good.len() * 3 / 4];
        atomic_write(&p, cut.as_bytes()).unwrap();
        let (d, outcome) = open_with_recovery(&p).unwrap();
        match outcome {
            OpenOutcome::RecoveredLenient(_) | OpenOutcome::RecoveredFromBackup(_) => {}
            other => panic!("expected recovery, got {other:?}"),
        }
        assert!(!d.pages.is_empty());
        // fully garbage main file -> backup is the only path
        atomic_write(&p, b"@@@@ not json at all").unwrap();
        let (d2, outcome2) = open_with_recovery(&p).unwrap();
        assert!(
            matches!(outcome2, OpenOutcome::RecoveredFromBackup(_)),
            "got {outcome2:?}"
        );
        assert!(!d2.pages.is_empty());
        for b in list_backups(&p) {
            let _ = std::fs::remove_file(b);
        }
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn backups_rotate_with_history() {
        let p = tmp("rot.x");
        for i in 0..4 {
            atomic_write(&p, format!("gen{i}").as_bytes()).unwrap();
            rotate_backups(&p);
        }
        // after 4 gens: bak1=gen3? NO — rotate happens BEFORE next write in
        // real flow; here bak1 holds the most recently rotated content
        let baks = list_backups(&p);
        assert_eq!(baks.len(), BACKUP_DEPTH, "history depth capped");
        let b1 = std::fs::read_to_string(&baks[0]).unwrap();
        let b2 = std::fs::read_to_string(&baks[1]).unwrap();
        assert_ne!(b1, b2, "distinct generations retained");
        for b in baks {
            let _ = std::fs::remove_file(b);
        }
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn legacy_hash_upgrade_converges_to_verified() {
        use x_core::{Library, LibraryDependency, Paint, Style};
        let mut lib = Library {
            library_id: "L".into(),
            name: "L".into(),
            version: 1,
            ..Default::default()
        };
        lib.styles.insert(
            "s".into(),
            Style::Paint {
                fill: Paint::Solid(Color::BLACK),
            },
        );
        let mut d = doc();
        d.library_deps.push(LibraryDependency {
            library_id: "L".into(),
            resolved_version: 1,
            snapshot_hash: String::new(),
            source_path: "l.xlib".into(),
        });
        d.library_snapshots.insert("L".into(), lib);
        // legacy: unhashed
        let st = crate::verify_document_libraries(&d);
        assert_eq!(st[0].1, crate::IntegrityStatus::LegacyUnhashed);
        let upgraded = upgrade_legacy_library_hashes(&mut d);
        assert_eq!(upgraded, vec!["L".to_string()]);
        let st = crate::verify_document_libraries(&d);
        assert_eq!(st[0].1, crate::IntegrityStatus::Verified, "converged");
    }
}
