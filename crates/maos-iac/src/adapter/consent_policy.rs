#![forbid(unsafe_code)]

//! Story 21-7 — the production consent policy: one registry of each admitted Spirit's declared
//! `[consent.recall]` grants, read by both halves of the control. The granter stamps the
//! daemon-minted envelope on the sender's outbound frame; the gate refuses a local addressee the
//! sender did not grant. A technical recall/delivery restriction, not legal consent.
//!
//! A declaration is bound to the admitted pid it was registered for, and a stamped frame carries
//! its OWN snapshot of it to the gate (`inflight`, released by the sender's seam), so a reload
//! between the stamp and the gate can neither widen nor mix the verdicts. A frame never stamped
//! (root or peer traffic) is not gated.

use std::collections::BTreeMap;
use std::sync::Arc;

use dashmap::DashMap;
use maos_domain::frame::{
    consent_wall_now_ns, ConsentEnvelope, ConsentRecall, FrameAddress, IacFrame, RuptureReason,
};

use super::mailbox::ConsentGate;

type Grants = Arc<BTreeMap<String, ConsentRecall>>;

/// Declared grants by sender spirit id. A sender with no entry declares nothing: its frames carry
/// no envelope and the gate accepts every addressee (today's behaviour).
#[derive(Debug, Default)]
pub struct ConsentPolicy {
    grants: DashMap<String, (u32, Grants)>,
    /// Frame id → the declaration its sender had when the frame was stamped (`None`: none).
    inflight: DashMap<[u8; 16], Option<Grants>>,
}

/// How restrictive a recall is: `Full` < `HeaderOnly` < `Withheld` (the enum's own order).
fn restriction(recall: ConsentRecall) -> u8 {
    recall as u8
}

impl ConsentPolicy {
    /// Register the admitted incarnation `pid`'s declaration; `None` clears the id's entry.
    pub fn register(
        &self,
        spirit_id: &str,
        pid: u32,
        grants: Option<BTreeMap<String, ConsentRecall>>,
    ) {
        match grants {
            Some(grants) => drop(
                self.grants
                    .insert(spirit_id.to_owned(), (pid, Arc::new(grants))),
            ),
            None => drop(self.grants.remove(spirit_id)),
        }
    }

    /// Withdraw the declaration only if it still belongs to incarnation `pid`: a stale cleanup
    /// can never erase a replacement's.
    pub fn unregister(&self, spirit_id: &str, pid: u32) {
        self.grants
            .remove_if(spirit_id, |_, (owner, _)| *owner == pid);
    }

    /// Replace `frame.consent_envelope` with the one the sender's declaration implies (or none)
    /// and pin that declaration to the frame for the gate. Whatever a guest supplied is
    /// discarded. The granter is `frame.from`, bound by the caller to the authenticated sender
    /// admitted as `pid`; a declaration owned by another pid refuses everything. With several
    /// addressees the one envelope carries the strictest recall among those granted
    /// (`Withheld` when none is). Returns whether every addressee is refused.
    pub fn stamp(&self, frame: &mut IacFrame, pid: u32) -> bool {
        let snapshot = self.grants.get(frame.from.spirit_id.as_str()).map(|entry| {
            let (owner, grants) = entry.value();
            if *owner == pid {
                Arc::clone(grants)
            } else {
                Grants::default()
            }
        });
        self.inflight.insert(frame.frame_id, snapshot.clone());
        let granted = |to: &FrameAddress| {
            let local = to.host_id.is_none();
            snapshot
                .as_ref()
                .filter(|_| local)
                .and_then(|g| g.get(to.spirit_id.as_str()).copied())
        };
        let recall = frame
            .to
            .iter()
            .filter_map(granted)
            .max_by_key(|r| restriction(*r));
        frame.consent_envelope = snapshot.is_some().then(|| ConsentEnvelope {
            consent_id: ulid::Ulid::new().to_bytes(),
            granter: FrameAddress {
                spirit_id: frame.from.spirit_id.clone(),
                host_id: None,
                role: None,
            },
            timestamp_ns: consent_wall_now_ns(),
            intent_class: None,
            valid_until_ns: None,
            recall: recall.unwrap_or(ConsentRecall::Withheld),
        });
        let all = frame.to.iter().all(|to| granted(to).is_none());
        snapshot.is_some() && !frame.to.is_empty() && all
    }

    /// Drop a stamped frame's pinned declaration once its delivery has been decided.
    pub fn release(&self, frame_id: [u8; 16]) {
        self.inflight.remove(&frame_id);
    }

    /// Whether `new` (a successor's declaration) would widen the live declaration of `spirit_id`.
    pub fn widened_by(
        &self,
        spirit_id: &str,
        new: Option<&BTreeMap<String, ConsentRecall>>,
    ) -> Option<String> {
        let old = self
            .grants
            .get(spirit_id)
            .map(|entry| entry.value().1.as_ref().clone());
        Self::widening(old.as_ref(), new)
    }

    /// A swap may only keep or narrow consent: the successor must declare when the predecessor
    /// did, grant no addressee the predecessor did not, and no wider recall
    /// (`Withheld` < `HeaderOnly` < `Full`). Returns what widens.
    pub fn widening(
        old: Option<&BTreeMap<String, ConsentRecall>>,
        new: Option<&BTreeMap<String, ConsentRecall>>,
    ) -> Option<String> {
        let old = old?;
        let Some(new) = new else {
            return Some("omits the declaration".into());
        };
        let wider = |(to, recall): &(&String, &ConsentRecall)| {
            old.get(*to)
                .is_none_or(|was| restriction(**recall) < restriction(*was))
        };
        new.iter()
            .find(wider)
            .map(|(to, _)| format!("widens the grant to {to}"))
    }
}

impl ConsentGate for ConsentPolicy {
    fn evaluate(&self, frame: &IacFrame, recipient: &FrameAddress) -> Result<(), RuptureReason> {
        // Only a frame stamped for an authenticated, admitted local Spirit sender is gated. An
        // unstamped frame (the operator's root dispatch, a verified peer) is identified by a
        // string alone, so a same-named installed Spirit's declaration never applies to it.
        let snapshot = self
            .inflight
            .get(&frame.frame_id)
            .and_then(|entry| entry.value().clone());
        match snapshot {
            Some(grants)
                if recipient.host_id.is_some()
                    || !grants.contains_key(recipient.spirit_id.as_str()) =>
            {
                Err(RuptureReason::ConsentNotGranted)
            }
            _ => Ok(()),
        }
    }
}
