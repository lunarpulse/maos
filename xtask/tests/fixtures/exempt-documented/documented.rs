// Story 15-1 AC2 residual control (§10), the GREEN half: this fixture reuses
// the name of a REAL exemption site (`VerifiedImageLock`) so that, checked
// against the shipped docs/invariants/i9-exemptions.md, the gate passes only
// because Story 15-1's T4 register entry exists. Reverting T4 alone (deleting
// the register entry) must red this test — the leg 15-1 closes stays closed.
#[i9_exempt(reason = "fixture; documented via the real register entry")]
pub struct VerifiedImageLock {
    attestations: Vec<u8>,
}
