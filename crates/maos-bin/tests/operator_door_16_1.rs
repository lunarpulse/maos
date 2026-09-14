use std::sync::Arc;
use std::time::Duration;

use maos_bin::operator_door::SpiritCommandLocks;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn production_spirit_locks_serialize_one_spirit_without_blocking_another() {
    let locks = Arc::new(SpiritCommandLocks::default());
    let butler_lock = locks.spirit_lock("butler");
    assert!(
        Arc::ptr_eq(&butler_lock, &locks.spirit_lock("butler")),
        "one Spirit id must always resolve to one serialization mutex"
    );
    assert!(
        !Arc::ptr_eq(&butler_lock, &locks.spirit_lock("researcher")),
        "different Spirits must not share a serialization mutex"
    );

    let held = butler_lock.lock().await;
    let waiting_lock = locks.spirit_lock("butler");
    let (acquired_tx, mut acquired_rx) = tokio::sync::oneshot::channel();
    let waiter = tokio::spawn(async move {
        let _guard = waiting_lock.lock().await;
        let _ = acquired_tx.send(());
    });

    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut acquired_rx)
            .await
            .is_err(),
        "a second command for the same Spirit entered while the first held the lock"
    );
    let researcher_lock = locks.spirit_lock("researcher");
    assert!(
        tokio::time::timeout(Duration::from_millis(50), researcher_lock.lock())
            .await
            .is_ok(),
        "an unrelated Spirit was serialized behind the held lock"
    );

    drop(held);
    tokio::time::timeout(Duration::from_secs(1), &mut acquired_rx)
        .await
        .expect("same-Spirit waiter should enter after release")
        .expect("waiter should signal acquisition");
    waiter.await.expect("waiter task");
}

#[cfg(unix)]
#[test]
fn symlinked_home_locks_the_target_directory_not_the_alias_parent() {
    const CHILD: &str = "MAOS_STORE_LOCK_SYMLINK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let root = std::path::PathBuf::from(
            std::env::var_os("MAOS_STORE_LOCK_TEST_ROOT").expect("child test root"),
        );
        let target_home = root.join("target-home");
        let locks = maos_bin::operator_door::acquire_store_lock_set(
            &root.join("stores/audit.sqlite"),
            maos_bin::operator_door::StoreLockRole::RootShared,
        )
        .expect("acquire canonical store lock set");
        assert!(
            locks
                .locked_paths()
                .contains(&std::fs::canonicalize(&target_home).expect("canonical target home")),
            "the MAOS_HOME symlink target itself was not locked: {:?}",
            locks.locked_paths()
        );
        return;
    }

    let temp = tempfile::TempDir::new().expect("tempdir");
    let root = temp.path();
    let target_home = root.join("target-home");
    let stores = root.join("stores");
    std::fs::create_dir_all(&target_home).expect("target home");
    std::fs::create_dir_all(&stores).expect("store root");
    let alias = root.join("home-alias");
    std::os::unix::fs::symlink(&target_home, &alias).expect("home symlink");

    let status = std::process::Command::new(std::env::current_exe().expect("current test binary"))
        .args([
            "--exact",
            "symlinked_home_locks_the_target_directory_not_the_alias_parent",
            "--nocapture",
        ])
        .env_clear()
        .env(CHILD, "1")
        .env("MAOS_STORE_LOCK_TEST_ROOT", root)
        .env("MAOS_HOME", &alias)
        .env("MAOS_MEMORY_ROOT", stores.join("memory"))
        .env("MAOS_ARCHIVE_DIR", stores.join("archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", stores.join("proofs"))
        .status()
        .expect("spawn isolated lock probe");
    assert!(status.success(), "isolated lock probe failed: {status}");
}
