use crate::error::CohortError;
use crate::manifest::CohortManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutboundConsentContext {
    pub acting_role: String,
    pub manifest_version: u64,
}

pub(crate) fn accept_admits(
    manifest: &CohortManifest,
    sender_peer: &str,
    acting_role: Option<&str>,
    intent: &str,
    sender_version: Option<u64>,
) -> Result<(), CohortError> {
    let sender_version = sender_version.ok_or(CohortError::EConsentManifestVersionAbsent)?;
    let delta = sender_version.abs_diff(manifest.version);
    if delta > 1 {
        return Err(CohortError::ECohortManifestSkew {
            sender_version,
            receiver_version: manifest.version,
            delta,
        });
    }

    let acting_role = acting_role.ok_or(CohortError::EConsentActingRoleAbsent)?;
    let sender = manifest
        .members
        .iter()
        .find(|member| member.host_id == sender_peer)
        .ok_or_else(|| CohortError::EConsentPeerNotMember {
            direction: "accept".into(),
            peer: sender_peer.into(),
        })?;
    if !sender.roles.iter().any(|role| role == acting_role) {
        return Err(CohortError::EConsentRoleNotEntitled {
            peer: sender_peer.into(),
            role: acting_role.into(),
        });
    }

    if manifest.consent.accept.iter().any(|grant| {
        grant.peer == sender_peer && grant.role == acting_role && grant.intent == intent
    }) {
        Ok(())
    } else {
        Err(CohortError::EConsentTupleDenied {
            direction: "accept".into(),
            peer: sender_peer.into(),
            role: Some(acting_role.into()),
            intent: intent.into(),
        })
    }
}

pub(crate) fn send_context(
    manifest: &CohortManifest,
    local_host: &str,
    receiver_peer: &str,
    intent: &str,
) -> Result<OutboundConsentContext, CohortError> {
    let receiver = manifest
        .members
        .iter()
        .find(|member| member.host_id == receiver_peer)
        .ok_or_else(|| CohortError::EConsentPeerNotMember {
            direction: "send".into(),
            peer: receiver_peer.into(),
        })?;
    let send_granted = manifest.consent.send.iter().any(|grant| {
        grant.peer == receiver_peer
            && receiver.roles.iter().any(|role| role == &grant.role)
            && grant.intent == intent
    });
    if !send_granted {
        return Err(CohortError::EConsentTupleDenied {
            direction: "send".into(),
            peer: receiver_peer.into(),
            role: None,
            intent: intent.into(),
        });
    }

    let local = manifest
        .members
        .iter()
        .find(|member| member.host_id == local_host)
        .ok_or_else(|| CohortError::EConsentPeerNotMember {
            direction: "send".into(),
            peer: local_host.into(),
        })?;
    let mut acting_role = None;
    for grant in manifest.consent.accept.iter().filter(|grant| {
        grant.peer == local_host
            && grant.intent == intent
            && local.roles.iter().any(|role| role == &grant.role)
    }) {
        match acting_role {
            None => acting_role = Some(grant.role.as_str()),
            Some(role) if role == grant.role => {}
            Some(_) => {
                return Err(CohortError::EConsentActingRoleAmbiguous {
                    peer: local_host.into(),
                    intent: intent.into(),
                });
            }
        }
    }
    let acting_role = acting_role.ok_or(CohortError::EConsentActingRoleAbsent)?;

    Ok(OutboundConsentContext {
        acting_role: acting_role.into(),
        manifest_version: manifest.version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{
        CohortAuthority, CohortMember, ConsentMatrix, ConsentTuple, ManifestSignature,
        RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE, SCHEMA_VERSION, T_STALE_DEFAULT,
    };

    const INTENT: &str = "cohort-work:write";

    fn manifest() -> CohortManifest {
        CohortManifest {
            schema_version: SCHEMA_VERSION,
            cohort_id: "consent-test".into(),
            version: 4,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![],
            },
            members: vec![
                CohortMember {
                    host_id: "host-a".into(),
                    fingerprint: format!("sha256:{}", "aa".repeat(32)),
                    roles: vec!["architect".into(), "reviewer".into()],
                },
                CohortMember {
                    host_id: "host-b".into(),
                    fingerprint: format!("sha256:{}", "bb".repeat(32)),
                    roles: vec!["receiver".into()],
                },
            ],
            consent: ConsentMatrix {
                send: vec![ConsentTuple {
                    peer: "host-b".into(),
                    role: "receiver".into(),
                    intent: INTENT.into(),
                }],
                accept: vec![ConsentTuple {
                    peer: "host-a".into(),
                    role: "architect".into(),
                    intent: INTENT.into(),
                }],
            },
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.into(),
                RESERVED_INTENT_HALT_RECEIPT.into(),
            ],
            t_stale_secs: T_STALE_DEFAULT,
            signature: ManifestSignature { sig: String::new() },
        }
    }

    #[test]
    fn accept_requires_the_frame_carried_role_exactly() {
        let manifest = manifest();
        assert!(accept_admits(&manifest, "host-a", Some("architect"), INTENT, Some(4)).is_ok());
        assert!(matches!(
            accept_admits(&manifest, "host-a", Some("reviewer"), INTENT, Some(4)),
            Err(CohortError::EConsentTupleDenied { .. })
        ));
    }

    #[test]
    fn accept_rejects_a_role_the_sender_does_not_hold() {
        assert!(matches!(
            accept_admits(&manifest(), "host-a", Some("operator"), INTENT, Some(4)),
            Err(CohortError::EConsentRoleNotEntitled { ref peer, ref role })
                if peer == "host-a" && role == "operator"
        ));
    }

    #[test]
    fn accept_fails_closed_when_role_or_version_is_absent() {
        assert!(matches!(
            accept_admits(&manifest(), "host-a", None, INTENT, Some(4)),
            Err(CohortError::EConsentActingRoleAbsent)
        ));
        assert!(matches!(
            accept_admits(&manifest(), "host-a", Some("architect"), INTENT, None),
            Err(CohortError::EConsentManifestVersionAbsent)
        ));
    }

    #[test]
    fn accept_reports_manifest_skew_as_its_own_cause() {
        assert!(matches!(
            accept_admits(&manifest(), "host-a", Some("architect"), INTENT, Some(2)),
            Err(CohortError::ECohortManifestSkew {
                sender_version: 2,
                receiver_version: 4,
                delta: 2,
            })
        ));
        assert!(accept_admits(&manifest(), "host-a", Some("architect"), INTENT, Some(3)).is_ok());
        assert!(accept_admits(&manifest(), "host-a", Some("architect"), INTENT, Some(5)).is_ok());
    }

    #[test]
    fn send_uses_the_send_table_and_derives_one_local_acting_role() {
        let context = send_context(&manifest(), "host-a", "host-b", INTENT).unwrap();
        assert_eq!(context.acting_role, "architect");
        assert_eq!(context.manifest_version, 4);
    }
}
