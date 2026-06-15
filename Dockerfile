# Story 9.4 AC-2 — Container image for MAOS.
#
# Multi-stage build: Rust builder → minimal runtime image.
# Push to Docker Hub + GHCR with cosign signing for verification
# parity with AC-1 binary signing.
#
# Build: docker build -t maos:v0.5.0 .
# Run:   docker run --rm maos:v0.5.0 --help
#
# The container image uses a distroless base for minimal attack surface.
# The MAOS binary is the sole entrypoint — no shell, no package manager.

# ── Builder stage ───────────────────────────────────────────────────────
FROM rust:1.88-bookworm AS builder

WORKDIR /build

# Cache dependencies by copying manifests first.
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY spirits/ spirits/
COPY examples/ examples/
COPY xtask/ xtask/
COPY templates/ templates/

# Build release binary with locked deps.
RUN cargo build --release -p maos-bin --locked

# ── Runtime stage ───────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /build/target/release/maos /usr/local/bin/maos

# Default data directory for Transparency Log + state.
ENV MAOS_HOME=/var/lib/maos

# The TL + journal directories are created at runtime by the binary.
VOLUME ["/var/lib/maos"]

ENTRYPOINT ["/usr/local/bin/maos"]
