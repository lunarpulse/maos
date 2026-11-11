import sys,re
root=sys.argv[1]; variant=sys.argv[2]
inf=f'{root}/crates/maos-kernel-core/src/inference/mod.rs'
s=open(inf).read()
a=s.index('                // F8 — principal attribution at emission (R2 / SR-4).')
b=s.index('                let mut dimensions = std::collections::BTreeMap::new();')
new='''                // Story 20-14 — FR64: the scope the root bound to the verified token at
                // mint; never read from the request (anti-spoof).
                let scope = self
                    .capabilities
                    .COSTREAD(&req.capability_token.token_id)
                    .unwrap_or_default();
                // F8 — principal attribution at emission (R2 / SR-4): the bound session
                // principal, else the write-target proxy as the tagged fallback.
                use maos_domain::cost::{AttributionConfidence, AttributionSource, PrincipalRef};
                let (principal, confidence, source) = match scope.session_principal {
                    Some(principal_id) => (
                        PrincipalRef::Resolved { principal_id },
                        AttributionConfidence::Exact,
                        AttributionSource::SessionPrincipal,
                    ),
                    None => {
                        let (principal, confidence) = match self
                            .transparency_log
                            .principal_ids_for_spirit_pid(req.spirit_pid)
                            .unwrap_or_default()
                            .as_slice()
                        {
                            [] => (PrincipalRef::Unattributed, AttributionConfidence::Unknown),
                            [one] => (
                                PrincipalRef::Resolved {
                                    principal_id: one.clone(),
                                },
                                AttributionConfidence::Exact,
                            ),
                            many => (
                                PrincipalRef::Ambiguous {
                                    count: many.len() as u32,
                                },
                                AttributionConfidence::Ambiguous,
                            ),
                        };
                        (principal, confidence, AttributionSource::WriteTargetProxy)
                    }
                };

'''
costread='cost_scope' if variant=='A' else 'tokens().cost_scope'
s=s[:a]+new.replace('.COSTREAD(','.'+costread+'(')+s[b:]
old=s[s.index('                let cost_payload = maos_domain::cost::CostAttributionPayload {'):s.index('                if let Ok(cost_bytes) = serde_json::to_vec(&cost_payload) {')]
newp='''                let cost_payload = maos_domain::cost::CostFrameBody {
                    payload: maos_domain::cost::CostAttributionPayload {
                        schema_version: 2,
                        timestamp_ns: now_ns,
                        spirit_pid: req.spirit_pid,
                        provider: resp.provider_attribution.provider_id.clone(),
                        model: resp
                            .provider_attribution
                            .model_id
                            .clone()
                            .unwrap_or_default(),
                        principal,
                        attribution_source: source,
                        attribution_confidence: confidence,
                        dimensions,
                    },
                    task_ref: scope.task_ref,
                };
'''
s=s.replace(old,newp)
open(inf,'w').write(s)
cap=f'{root}/crates/maos-kernel-core/src/capability/mod.rs'
c=open(cap).read()
if variant=='A':
    c=c.replace('''    quarantined: std::sync::RwLock<std::collections::BTreeSet<u32>>,
}''','''    quarantined: std::sync::RwLock<std::collections::BTreeSet<u32>>,
    /// Story 20-14 (FR64) — the attribution scope the root bound to a token at
    /// mint; read by the inference emission site, pruned with the token.
    cost_scopes:
        std::sync::RwLock<std::collections::HashMap<TokenId, maos_domain::cost::CostScope>>,
}''',1)
    c=c.replace('''            quarantined: std::sync::RwLock::default(),
        }''','''            quarantined: std::sync::RwLock::default(),
            cost_scopes: std::sync::RwLock::default(),
        }''',1)
    c=c.replace('''    /// Story 4.2 — write a tagged scalar to the per-Spirit slot store''','''    /// Story 20-14 — bind a root attribution scope to a minted token; scopes of
    /// tokens the ring no longer holds are pruned on the way in.
    pub fn bind_cost_scope(&self, token_id: TokenId, scope: maos_domain::cost::CostScope) {
        let mut scopes = self.cost_scopes.write().unwrap_or_else(|e| e.into_inner());
        scopes.retain(|id, _| self.tokens.get_scope(id).is_some());
        scopes.insert(token_id, scope);
    }

    /// Story 20-14 — the scope bound to `token_id`, if any.
    pub fn cost_scope(&self, token_id: &TokenId) -> Option<maos_domain::cost::CostScope> {
        let scopes = self.cost_scopes.read().unwrap_or_else(|e| e.into_inner());
        scopes.get(token_id).cloned()
    }

    /// Story 4.2 — write a tagged scalar to the per-Spirit slot store''',1)
else:
    c=c.replace('''    /// Story 4.2 — write a tagged scalar to the per-Spirit slot store''','''    /// Story 20-14 — the cap-token ring; the root binds attribution scopes on it.
    pub fn tokens(&self) -> &CapTokensShardRing {
        &self.tokens
    }

    /// Story 4.2 — write a tagged scalar to the per-Spirit slot store''',1)
    rg=f'{root}/crates/maos-capability/src/cap_tokens/mod.rs'
    r=open(rg).read()
    r=r.replace('''    audit: cap_audit::Sender,
}

impl std::fmt::Debug for CapTokensShardRing''','''    audit: cap_audit::Sender,
    /// Story 20-14 — root-bound attribution scopes, keyed by the token they ride.
    cost_scopes: parking_lot::RwLock<std::collections::HashMap<TokenId, CostScope>>,
}

impl std::fmt::Debug for CapTokensShardRing''',1)
    r=r.replace('''            signing_key,
            boot_nonce,
            audit,
        }''','''            signing_key,
            boot_nonce,
            audit,
            cost_scopes: Default::default(),
        }''',1)
    r=r.replace('''    /// Revoke a single token. Slow-path''','''    /// Story 20-14 — bind a root attribution scope to a token; scopes of tokens
    /// the ring no longer holds are pruned on the way in.
    pub fn bind_cost_scope(&self, token_id: TokenId, scope: CostScope) {
        let mut scopes = self.cost_scopes.write();
        scopes.retain(|id, _| self.get_scope(id).is_some());
        scopes.insert(token_id, scope);
    }

    /// Story 20-14 — the scope bound to `token_id`, if any.
    pub fn cost_scope(&self, token_id: &TokenId) -> Option<CostScope> {
        self.cost_scopes.read().get(token_id).cloned()
    }

    /// Revoke a single token. Slow-path''',1)
    r=r.replace('use maos_domain::invariants::i1::','use maos_domain::cost::CostScope;\nuse maos_domain::invariants::i1::',1)
    open(rg,'w').write(r)
open(cap,'w').write(c)
