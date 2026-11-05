# Story 20-2: runtime contains exactly the signed release binary.
FROM debian:trixie-slim@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4 AS verify
RUN apt-get update && apt-get install -y --no-install-recommends python3 python3-cryptography && rm -rf /var/lib/apt/lists/*
WORKDIR /verify
ARG TARGETARCH
ARG RELEASE_PUBKEY
COPY packaging/verify-release.py ./verify-release.py
COPY dist/SHA256SUMS dist/SHA256SUMS.sig ./
COPY dist/maos-linux-${TARGETARCH} ./maos-linux-${TARGETARCH}
RUN python3 verify-release.py --pubkey "$RELEASE_PUBKEY" "maos-linux-${TARGETARCH}" && cp "maos-linux-${TARGETARCH}" maos && chmod 755 maos
RUN mkdir /state && chmod 755 /state

FROM gcr.io/distroless/cc-debian13:nonroot@sha256:08476c244888bfb425bae1ae4dd7c17f093e972aab8426883d97f3c6f280f7c1
ARG SOURCE_SHA
LABEL org.opencontainers.image.revision=$SOURCE_SHA
COPY --from=verify /verify/maos /usr/local/bin/maos
COPY --from=verify --chown=65532:65532 /state /var/lib/maos
ENV MAOS_HOME=/var/lib/maos
VOLUME ["/var/lib/maos"]
ENTRYPOINT ["/usr/local/bin/maos"]
