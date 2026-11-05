# Story 20-2: runtime contains exactly the signed release binary.
FROM debian:trixie-slim AS verify
RUN apt-get update && apt-get install -y --no-install-recommends python3 python3-cryptography && rm -rf /var/lib/apt/lists/*
WORKDIR /verify
ARG TARGETARCH
ARG RELEASE_PUBKEY
COPY packaging/verify-release.py ./verify-release.py
COPY dist/SHA256SUMS dist/SHA256SUMS.sig ./
COPY dist/maos-linux-${TARGETARCH} ./maos-linux-${TARGETARCH}
RUN python3 verify-release.py --pubkey "$RELEASE_PUBKEY" "maos-linux-${TARGETARCH}" && cp "maos-linux-${TARGETARCH}" maos && chmod 755 maos

FROM gcr.io/distroless/cc-debian13:nonroot
ARG SOURCE_SHA
LABEL org.opencontainers.image.revision=$SOURCE_SHA
COPY --from=verify /verify/maos /usr/local/bin/maos
ENV MAOS_HOME=/var/lib/maos
VOLUME ["/var/lib/maos"]
ENTRYPOINT ["/usr/local/bin/maos"]
