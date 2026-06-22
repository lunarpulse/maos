---
title: 핫스왑 마이그레이션
sidebar_position: 7
description: Spirit 버전 간 무중단 상태 전달을 위해 snapshot()과 migrate() 구현.
review_status: machine
---

# 핫스왑 마이그레이션

## Problem

새 버전의 Spirit을 배포하면서 중단 없이 이전 인스턴스에서 새 인스턴스로 진행 중인 상태를 전달해야 합니다. kernel의 핫스왑 프로토콜이 선행자에서 `snapshot()`을, 후속자에서 `migrate()`를 호출합니다 — 양쪽을 모두 구현해야 합니다.

## Solution

manifest에 핫스왑 지원을 선언합니다:

```toml
[hot_swap]
state_schema_version = 2

[migrates_from]
versions = ["1.0.0"]

[halt_protocol_compatibility]
version = 1
```

핫스왑 훅을 구현합니다:

```rust
use maos_spirit_abi::lifecycle::{Spirit, MigratorError, SwapInPayload};
use maos_spirit_abi::ctx::Ctx;

/// State envelope — versioned for forward compatibility.
#[derive(serde::Serialize, serde::Deserialize)]
struct StateSnapshot {
    schema_version: u32,
    counter: u64,
    buffer: Vec<u8>,
}

pub struct MySpirit {
    counter: std::cell::Cell<u64>,
    buffer: std::cell::RefCell<Vec<u8>>,
}

impl MySpirit {
    pub fn new() -> Self {
        Self {
            counter: std::cell::Cell::new(0),
            buffer: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl Spirit for MySpirit {
    /// Called on the OLD instance before swap-out.
    /// Flush in-flight work, release locks.
    fn on_swap_out(&self, ctx: &mut Ctx) {
        // Flush any pending writes before the kernel takes the snapshot.
    }

    /// Produce a CBOR-encoded state snapshot.
    /// The kernel passes this blob to the successor's on_swap_in / migrate.
    fn snapshot(&self, _ctx: &mut Ctx) -> Vec<u8> {
        let snap = StateSnapshot {
            schema_version: 2,
            counter: self.counter.get(),
            buffer: self.buffer.borrow().clone(),
        };
        // Use any codec — CBOR, bincode, JSON. The kernel treats it as
        // opaque bytes; the successor must understand the format.
        serde_json::to_vec(&snap).unwrap_or_default()
    }

    /// Called on the NEW instance when predecessor state arrives.
    fn on_swap_in<'a>(&self, ctx: &mut Ctx, payload: &SwapInPayload<'a>) {
        // on_swap_in receives the snapshot from the predecessor.
        // For same-version swaps, deserialise directly.
    }

    /// Cross-major migration: translate predecessor state to this version's schema.
    fn migrate(
        &self,
        _ctx: &mut Ctx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, MigratorError> {
        // Attempt to deserialise the predecessor's snapshot.
        let old: StateSnapshot = serde_json::from_slice(predecessor_state)
            .map_err(|e| MigratorError::DeserializationFailed(
                e.to_string().into()
            ))?;

        match old.schema_version {
            1 => {
                // Schema v1 -> v2: add the new buffer field.
                let migrated = StateSnapshot {
                    schema_version: 2,
                    counter: old.counter,
                    buffer: Vec::new(), // v1 had no buffer
                };
                serde_json::to_vec(&migrated)
                    .map_err(|e| MigratorError::SerializationFailed(
                        e.to_string().into()
                    ))
            }
            2 => {
                // Same schema — pass through.
                Ok(predecessor_state.to_vec())
            }
            v => Err(MigratorError::UnsupportedVersion(v)),
        }
    }
}
```

## Discussion

kernel이 오케스트레이션하는 핫스왑 시퀀스:

1. 선행자에서 **`on_swap_out`** — 상태를 플러시하고, 잠금을 해제합니다.
2. 선행자에서 **`snapshot`** — 직렬화된 상태의 바이트 블롭을 생성합니다.
3. 후속자에서 **`on_swap_in`** — 같은 버전 스왑을 위해 블롭을 받습니다.
4. 후속자에서 **`migrate`** — 크로스 버전 상태를 변환합니다(`[migrates_from]`이 선행자 버전을 나열할 때만 호출).

`[hot_swap].state_schema_version` 필드는 `snapshot()`이 생성하는 상태 블롭의 버전입니다. `[migrates_from].versions` 필드는 `migrate()`가 처리할 수 있는 선행자 버전을 선언합니다.

**설계 규칙:**

- 스냅샷 포맷을 항상 버전화하세요. 직렬화된 블롭에 `schema_version` 필드를 넣어 `migrate()`가 분기할 수 있게 합니다.
- Spirit이 마이그레이션을 지원하지 않으면 `MigratorError::NotImplemented`(기본값)를 반환하세요. kernel이 클린 시작으로 폴백합니다.
- `MigratorError::UnsupportedVersion(v)`는 kernel에게 선행자 버전이 마이그레이션하기에 너무 오래되었다고 알립니다 — 운영자가 명확한 진단을 받습니다.
- 스냅샷을 작게 유지하세요. kernel은 스왑 윈도우 동안 블롭을 메모리에 보관합니다. 큰 상태는 스냅샷에 참조만 담아 외부에 저장해야 합니다.
