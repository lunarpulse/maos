#![forbid(unsafe_code)]

//! j1-crosshost-2b §A6 review P4 — the AC3.2 negative that was missing from the
//! dev pass: the duplicate classification must fire ONLY on the extended
//! primary-key/unique codes, never on the broader `SQLITE_CONSTRAINT` primary
//! code that `ErrorCode::ConstraintViolation` carries (which also covers
//! `NOT NULL`, `CHECK` and `FOREIGN KEY`).
//!
//! Why this is the load-bearing control: `transparency_log` declares `NOT NULL`
//! on ten of its twelve columns. If the arm were widened back to
//! `ErrorCode::ConstraintViolation`, a genuine NOT-NULL defect (silent audit
//! loss) would classify as "already journaled" — the exact inversion of
//! AC3.2's intent — and the duplicate-replay test in
//! `crates/maos-bin/tests/two_host_delegation_2b.rs` would stay green the whole
//! way down. Only this vector reds on that widening.

use maos_iac::transparency_log::TransparencyLogAdapter;
use rusqlite::ffi::Error as FfiError;
use rusqlite::{Error::SqliteFailure, ErrorCode};

fn constraint_failure(extended_code: i32) -> rusqlite::Error {
    SqliteFailure(
        FfiError {
            code: ErrorCode::ConstraintViolation,
            extended_code,
        },
        Some("constraint failed".into()),
    )
}

#[test]
fn duplicate_primary_key_and_unique_classify_as_duplicate() {
    assert!(
        TransparencyLogAdapter::is_duplicate_primary_key(&constraint_failure(
            rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
        )),
        "the primary-key extended code is the peer-replay case AC3.2 types"
    );
    assert!(
        TransparencyLogAdapter::is_duplicate_primary_key(&constraint_failure(
            rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
        )),
        "a unique-index violation is the same already-present-row verdict"
    );
}

#[test]
fn not_null_violation_is_not_a_duplicate() {
    // THE mandated negative (AC3.2): a NOT NULL violation carries the SAME
    // primary code (`SQLITE_CONSTRAINT` → `ErrorCode::ConstraintViolation`)
    // but a different extended code. Matching on the primary code — the exact
    // widening this test forbids — would silently convert genuine audit loss
    // into `FrameRowWrite::Duplicate`.
    let not_null = constraint_failure(rusqlite::ffi::SQLITE_CONSTRAINT_NOTNULL);
    assert_eq!(
        match &not_null {
            SqliteFailure(e, _) => e.code,
            _ => unreachable!("constructed as SqliteFailure"),
        },
        ErrorCode::ConstraintViolation,
        "precondition: NOT NULL shares the primary code with the duplicate arm"
    );
    assert!(
        !TransparencyLogAdapter::is_duplicate_primary_key(&not_null),
        "a NOT NULL violation must fall through to the I2 panic — it is audit \
         loss, not an already-journaled row"
    );
}

#[test]
fn check_foreign_key_and_non_sqlite_errors_are_not_duplicates() {
    // The remaining constraint-flavoured extended codes, and every non-sqlite
    // error shape, must also fall through: only the two extended codes above
    // mean "the row is already there".
    assert!(!TransparencyLogAdapter::is_duplicate_primary_key(
        &constraint_failure(rusqlite::ffi::SQLITE_CONSTRAINT_CHECK)
    ));
    assert!(!TransparencyLogAdapter::is_duplicate_primary_key(
        &constraint_failure(rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY)
    ));
    assert!(!TransparencyLogAdapter::is_duplicate_primary_key(
        &rusqlite::Error::QueryReturnedNoRows
    ));
    assert!(!TransparencyLogAdapter::is_duplicate_primary_key(
        &SqliteFailure(
            FfiError {
                code: ErrorCode::DatabaseCorrupt,
                extended_code: rusqlite::ffi::SQLITE_CORRUPT,
            },
            None,
        )
    ));
}
