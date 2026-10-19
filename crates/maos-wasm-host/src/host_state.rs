//! Wasmtime `Store<T>` state for the contained runner.
//!
//! `wasip2`-targeted components (built via `cargo component`/`wit-bindgen`
//! with the `wasm32-wasip2` target, as `guests/echo-spirit` is) import
//! `wasi:cli`/`wasi:io` interfaces even when the guest code itself never
//! calls them — the Rust std runtime startup pulls them in. `HostState` +
//! `wasmtime_wasi::p2::add_to_linker_sync` satisfy those imports with a
//! minimal WASI context. The runner runs inside ADR-031's T2 process boundary
//! once launched by `spawn_and_bridge`; the production launch is
//! `17-3c-wasm-spirit-on-the-bus-under-t2`. WASI is not that security boundary.

use std::io::{BufReader, BufWriter};

use crate::recall_wire::{self as wire, RecallReply, RecallRequest, WireBytes, WireError};
use crate::wit_guest::maos::spirit::recall as wit;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

pub struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    pub limits: wasmtime::StoreLimits,
    // The frame pump and nested host imports MUST share this one buffer pair.
    // Replacing the reader would discard already-prefetched inbound bytes.
    pub reader: BufReader<std::io::Stdin>,
    pub writer: BufWriter<std::io::Stdout>,
    pub in_export: bool,
    pub recall_broken: bool,
    recall_closed: bool,
}

impl HostState {
    /// A minimal WASI context: no preopened directories, no network, no
    /// inherited env/args. The guest gets only what the WIT world's
    /// `handle-frame`/`on-start`/`on-shutdown` exports need — nothing.
    pub fn new() -> Self {
        Self {
            ctx: WasiCtxBuilder::new()
                .stdout(std::io::stderr())
                .stderr(std::io::stderr())
                .build(),
            table: ResourceTable::new(),
            reader: BufReader::new(std::io::stdin()),
            writer: BufWriter::new(std::io::stdout()),
            in_export: false,
            recall_broken: false,
            recall_closed: false,
            // `memory_size` caps each linear memory; one memory makes 64 MiB
            // the aggregate guest cap (Spirit components define one memory).
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size(64 * 1024 * 1024)
                .instances(16)
                .memories(1)
                .tables(16)
                .table_elements(65_536)
                .build(),
        }
    }

    fn exchange(&mut self, request: RecallRequest) -> Result<RecallReply, wit::RecallError> {
        if !self.in_export || self.recall_broken || self.recall_closed {
            return Err(wit::RecallError::Unavailable);
        }
        let body = wire::encode_request(&request).map_err(|_| wit::RecallError::Unavailable)?;
        if maos_frame_codec::write_frame(&mut self.writer, &body).is_err() {
            return Err(self.desync());
        }
        match maos_frame_codec::read_frame(&mut self.reader) {
            Ok(None) => {
                self.recall_closed = true;
                Err(wit::RecallError::Unavailable)
            }
            Ok(Some(body)) => wire::decode_reply(&body).map_err(|_| self.desync()),
            Err(_) => Err(self.desync()),
        }
    }

    fn desync(&mut self) -> wit::RecallError {
        self.recall_broken = true;
        wit::RecallError::Unavailable
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl wit::Host for HostState {
    fn recall(&mut self, filter: wit::RecallFilter) -> Result<wit::RecallPage, wit::RecallError> {
        let request = RecallRequest::Recall {
            filter: wire::WireFilter {
                kind: filter.kind,
                since_ns: filter.since_ns,
                until_ns: filter.until_ns,
                limit: filter.limit,
                intent_filter: filter.intent_filter,
                cursor: filter.cursor.map(|cursor| wire::WireCursor {
                    last_timestamp_ns: cursor.last_timestamp_ns,
                    last_frame_id: WireBytes(cursor.last_frame_id),
                }),
            },
        };
        match self.exchange(request)? {
            RecallReply::Page {
                entries,
                next_cursor,
            } => {
                if entries.iter().any(|entry| entry.frame_id.0.len() != 16)
                    || next_cursor
                        .as_ref()
                        .is_some_and(|cursor| cursor.last_frame_id.0.len() != 16)
                {
                    return Err(self.desync());
                }
                Ok(wit::RecallPage {
                    entries: entries
                        .into_iter()
                        .map(|entry| wit::RecallEntry {
                            frame_id: entry.frame_id.0,
                            timestamp_ns: entry.timestamp_ns,
                            kind: entry.kind,
                            intent: entry.intent,
                            payload_available: entry.payload_available,
                        })
                        .collect(),
                    next_cursor: next_cursor.map(|cursor| wit::RecallCursor {
                        last_timestamp_ns: cursor.last_timestamp_ns,
                        last_frame_id: cursor.last_frame_id.0,
                    }),
                })
            }
            RecallReply::Error(error) => Err(error.into()),
            _ => Err(self.desync()),
        }
    }

    fn fetch(&mut self, frame_id: Vec<u8>) -> Result<wit::FetchResponse, wit::RecallError> {
        match self.exchange(RecallRequest::Fetch {
            frame_id: WireBytes(frame_id),
        })? {
            RecallReply::Fetched {
                frame_id,
                timestamp_ns,
                kind,
                intent,
                payload,
                origin,
            } => {
                use crate::wit_guest::maos::spirit::frames::FrameOrigin;
                let origin = match origin.as_str() {
                    "human-authored" => FrameOrigin::HumanAuthored,
                    "spirit-auto" => FrameOrigin::SpiritAuto,
                    "spirit-drafted-human-approved" => FrameOrigin::SpiritDraftedHumanApproved,
                    "kernel" => FrameOrigin::Kernel,
                    _ => return Err(self.desync()),
                };
                if frame_id.0.len() != 16 {
                    return Err(self.desync());
                }
                Ok(wit::FetchResponse {
                    frame_id: frame_id.0,
                    timestamp_ns,
                    kind,
                    intent,
                    payload: payload.0,
                    origin,
                })
            }
            RecallReply::Error(error) => Err(error.into()),
            _ => Err(self.desync()),
        }
    }
}

impl From<WireError> for wit::RecallError {
    fn from(error: WireError) -> Self {
        match error {
            WireError::ScopeViolation => Self::ScopeViolation,
            WireError::FrameNotFound => Self::FrameNotFound,
            WireError::InvalidRequest(detail) => Self::InvalidRequest(detail),
            WireError::LimitExceeded(limit) => Self::LimitExceeded(limit),
            WireError::CapabilityDenied => Self::CapabilityDenied,
            WireError::ConsentWithheld => Self::ConsentWithheld,
            WireError::ReplyTooLarge(bound) => Self::ReplyTooLarge(bound),
            WireError::Unavailable => Self::Unavailable,
        }
    }
}
