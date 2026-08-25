use crate::{Cva, CvaReconcileError, CvaReconcileResult};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

impl Cva {
    pub fn reconcile_and_promote(
        canonical_path: impl AsRef<Path>,
        conflicted_path: impl AsRef<Path>,
    ) -> Result<CvaReconcileResult, CvaReconcileError> {
        let canonical = canonical_path.as_ref();
        let conflicted = conflicted_path.as_ref();
        if fs::canonicalize(canonical)? == fs::canonicalize(conflicted)? {
            return Err(CvaReconcileError::PromotionPathsMustDiffer);
        }

        let before = fingerprint(canonical)?;
        let temp = sibling_path(canonical, "merge.tmp");
        let backup = sibling_path(canonical, "merge.bak");
        let result = Self::reconcile(canonical, conflicted, &temp)?;
        if !result.canonical_change_required {
            let _ = fs::remove_file(&temp);
            return Ok(result);
        }

        let promote_result = (|| {
            let candidate = Self::open(&temp)?;
            candidate.sync()?;
            drop(candidate);

            if fingerprint(canonical)? != before {
                return Err(CvaReconcileError::CanonicalChangedDuringPromotion);
            }

            fs::copy(canonical, &backup)?;
            sync_file(&backup)?;
            Self::open(&backup)?;
            if fingerprint(&backup)? != before {
                return Err(CvaReconcileError::CanonicalChangedDuringPromotion);
            }

            let parent = canonical.parent().unwrap_or_else(|| Path::new("."));
            atomic_replace(&temp, canonical)?;
            if let Err(failure) = finalize_promoted(canonical, parent) {
                let failure = failure.to_string();
                if let Err(recovery) = restore_backup(&backup, canonical, parent) {
                    return Err(CvaReconcileError::PromotionRecoveryFailed {
                        failure,
                        recovery: recovery.to_string(),
                    });
                }
                return Err(CvaReconcileError::PromotionFinalizationFailed(failure));
            }
            let _ = fs::remove_file(&backup);
            let _ = sync_parent(parent);
            Ok(())
        })();

        if let Err(error) = promote_result {
            let _ = fs::remove_file(&temp);
            let _ = fs::remove_file(&backup);
            return Err(error);
        }
        Ok(result)
    }
}

fn finalize_promoted(canonical: &Path, parent: &Path) -> Result<(), CvaReconcileError> {
    sync_parent(parent)?;
    let promoted = Cva::open(canonical)?;
    promoted.sync()?;
    Ok(())
}

fn restore_backup(backup: &Path, canonical: &Path, parent: &Path) -> Result<(), CvaReconcileError> {
    atomic_replace(backup, canonical)?;
    sync_parent(parent)?;
    let restored = Cva::open(canonical)?;
    restored.sync()?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileFingerprint {
    len: u64,
    digest: [u8; 32],
}

fn fingerprint(path: &Path) -> Result<FileFingerprint, CvaReconcileError> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut len = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        len = len.saturating_add(read as u64);
        hash.update(&buffer[..read]);
    }
    Ok(FileFingerprint {
        len,
        digest: hash.finalize().into(),
    })
}

fn sync_file(path: &Path) -> Result<(), CvaReconcileError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?
        .sync_all()?;
    Ok(())
}

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace.cva");
    path.with_file_name(format!(".{name}.{}.{}.{suffix}", std::process::id(), stamp))
}

#[cfg(not(windows))]
fn atomic_replace(from: &Path, to: &Path) -> Result<(), CvaReconcileError> {
    fs::rename(from, to)?;
    Ok(())
}

#[cfg(windows)]
fn atomic_replace(from: &Path, to: &Path) -> Result<(), CvaReconcileError> {
    use std::os::windows::ffi::OsStrExt;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    let ok = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> Result<(), CvaReconcileError> {
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> Result<(), CvaReconcileError> {
    Ok(())
}
