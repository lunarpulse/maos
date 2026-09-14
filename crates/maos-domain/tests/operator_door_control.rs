use std::sync::{Arc, Barrier};

use maos_domain::operator_door::{
    control_file_path, ensure_control_file, ControlFile, ControlFileState,
};

#[test]
fn concurrent_initializers_publish_one_complete_control_file() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let home = temp.path().join("home");
    let barrier = Arc::new(Barrier::new(8));
    let mut threads = Vec::new();

    for _ in 0..8 {
        let home = home.clone();
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            ensure_control_file(&home).expect("concurrent initializer")
        }));
    }

    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().expect("initializer thread"))
        .collect();
    assert_eq!(
        results
            .iter()
            .filter(|(_, state)| *state == ControlFileState::Minted)
            .count(),
        1,
        "exactly one initializer must publish the no-replace destination"
    );

    let persisted = ControlFile::load(&control_file_path(&home)).expect("published control file");
    assert!(
        results.iter().all(|(control, _)| control == &persisted),
        "losing initializers must return the winner instead of overwriting it"
    );
}
