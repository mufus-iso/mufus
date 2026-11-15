#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(any(target_os = "macos", test))]
use std::sync::{Mutex, TryLockError};

#[cfg(target_os = "macos")]
static DISK_CHECK: Mutex<()> = Mutex::new(());

#[cfg(any(target_os = "macos", test))]
fn run_disk_check(
    lock: &Mutex<()>,
    check: impl FnOnce() -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    let _guard = match lock.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(error)) => error.into_inner(),
        Err(TryLockError::WouldBlock) => return Err("A disk check is already running."),
    };
    check()
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn check_disk_discovery() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        run_disk_check(&DISK_CHECK, || {
            mufus_core::macos::collect_external_snapshot()
                .map(|_| ())
                .map_err(|_| "The disk check failed. Check your devices and try again.")
        })
    })
    .await
    .map_err(|_| "The disk check could not be completed.".to_owned())?
    .map_err(str::to_owned)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn check_disk_discovery() -> Result<(), String> {
    Err("The disk check is currently available only on macOS.".to_owned())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![check_disk_discovery])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("Failed to start mufus: {error}");
            std::process::exit(1);
        });
}

#[cfg(test)]
mod tests {
    use super::run_disk_check;
    use std::sync::Mutex;

    #[test]
    fn rejects_overlapping_checks_and_releases_lock_after_failure() {
        let lock = Mutex::new(());
        assert_eq!(
            run_disk_check(&lock, || {
                assert_eq!(
                    run_disk_check(&lock, || Ok(())),
                    Err("A disk check is already running.")
                );
                Err("check failed")
            }),
            Err("check failed")
        );
        assert_eq!(run_disk_check(&lock, || Ok(())), Ok(()));
    }

    #[test]
    fn permits_retry_after_a_panicking_check() {
        let lock = Mutex::new(());
        let result = std::panic::catch_unwind(|| {
            let _ = run_disk_check(&lock, || panic!("check panicked"));
        });
        assert!(result.is_err());
        assert_eq!(run_disk_check(&lock, || Ok(())), Ok(()));
    }

    #[test]
    fn holds_lock_until_worker_finishes() -> Result<(), Box<dyn std::error::Error>> {
        let lock = Mutex::new(());
        std::thread::scope(|scope| {
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (finish_tx, finish_rx) = std::sync::mpsc::channel();
            let lock_ref = &lock;
            let worker = scope.spawn(move || {
                run_disk_check(lock_ref, || {
                    started_tx.send(()).map_err(|_| "start channel closed")?;
                    finish_rx.recv().map_err(|_| "finish channel closed")?;
                    Ok(())
                })
            });
            started_rx.recv_timeout(std::time::Duration::from_secs(2))?;
            assert_eq!(
                run_disk_check(&lock, || Ok(())),
                Err("A disk check is already running.")
            );
            finish_tx.send(())?;
            worker.join().map_err(|_| "worker panicked")??;
            assert_eq!(run_disk_check(&lock, || Ok(())), Ok(()));
            Ok(())
        })
    }
}
