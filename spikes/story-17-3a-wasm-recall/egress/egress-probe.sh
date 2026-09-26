#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
egress_dir="$repo_root/spikes/story-17-3a-wasm-recall/egress"
image="maos-story-17-3a-egress-probe:local"
runner_image=${RUNNER_IMAGE:-local}
base=(--rm --user=65534:65534 --read-only)
contradictions=0

notice() {
    printf '::notice title=egress-probe/%s::%s rc=%s sig=0\n' "$runner_image" "$1" "$2"
}

contradiction() {
    printf '::error title=egress-probe/%s::%s rc=%s sig=0\n' "$runner_image" "$1" "$2"
    contradictions=$((contradictions + 1))
}

printf 'UTC %s\n' "$(date -u +%FT%TZ)"
printf '+ git rev-parse HEAD\n'
git -C "$repo_root" rev-parse HEAD
printf 'PROBE_INPUTS tree/runtime: podman, its rootless network backend, host-loopback responders, and real container network namespaces; not only probe-created state.\n'
printf '+ podman --version\n'
podman_version=$(podman --version)
printf '%s\n' "$podman_version"
notice "podman-version=${podman_version}" 0
printf '+ podman info --format {{.Host.NetworkBackend}}\n'
if network_backend=$(podman info --format '{{.Host.NetworkBackend}}'); then
    printf 'PODMAN_NETWORK_BACKEND=%s\n' "$network_backend"
    notice "podman-network-backend=${network_backend}" 0
else
    rc=$?
    printf 'PODMAN_UNAVAILABLE rc=%s; M1-M3 NOT-MEASURED on this runtime\n' "$rc"
    notice "podman-unavailable" "$rc"
    exit 0
fi
for backend_binary in slirp4netns pasta; do
    printf '+ %s --version\n' "$backend_binary"
    if backend_version=$("$backend_binary" --version 2>&1); then
        printf '%s_VERSION=%s\n' "$backend_binary" "$backend_version"
        notice "${backend_binary}-version=${backend_version}" 0
    else
        rc=$?
        printf '%s_VERSION=unavailable rc=%s\n' "$backend_binary" "$rc"
        notice "${backend_binary}-version=unavailable" "$rc"
    fi
done

workdir=$(mktemp -d)
tcp_port_file="$workdir/tcp-port"
unix_socket="$workdir/egress.sock"
tcp_pid=''
unix_pid=''
cleanup() {
    set +e
    if [ -n "$tcp_pid" ]; then
        kill "$tcp_pid"
        wait "$tcp_pid"
    fi
    if [ -n "$unix_pid" ]; then
        kill "$unix_pid"
        wait "$unix_pid"
    fi
    rm -rf "$workdir"
}
trap cleanup EXIT

printf '+ python3 host_http.py %s\n' "$tcp_port_file"
python3 "$egress_dir/host_http.py" "$tcp_port_file" &
tcp_pid=$!
for _ in $(seq 1 50); do
    if [ -s "$tcp_port_file" ]; then
        break
    fi
    sleep 0.1
done
if [ ! -s "$tcp_port_file" ]; then
    printf 'HOST_TCP_START_FAILED\n'
    exit 70
fi
tcp_port=$(cat "$tcp_port_file")
printf 'HOST_LOOPBACK=127.0.0.1:%s\n' "$tcp_port"

printf '+ podman build -f egress/Dockerfile.socat -t %s egress\n' "$image"
podman build -f "$egress_dir/Dockerfile.socat" -t "$image" "$egress_dir"

run_host_probe() {
    mechanism=$1
    network=$2
    endpoint=$3
    if output=$(podman run "${base[@]}" --network="$network" "$image" -ec "wget -qO- --timeout=5 '$endpoint'"); then
        host_rc=0
        host=yes
    else
        host_rc=$?
        host=no
    fi
    printf '%s_HOST=%s rc=%s output=%s\n' "$mechanism" "$host" "$host_rc" "$output"
}

run_internet_probe() {
    mechanism=$1
    network=$2
    if output=$(podman run "${base[@]}" --network="$network" "$image" -ec 'wget -qO- --timeout=5 https://example.com >/dev/null'); then
        internet_rc=0
        internet=yes
    else
        internet_rc=$?
        internet=no
    fi
    printf '%s_INTERNET=%s rc=%s output=%s\n' "$mechanism" "$internet" "$internet_rc" "$output"
}

printf '+ M1 podman run --user=65534:65534 --read-only --network=slirp4netns:allow_host_loopback=true ... http://10.0.2.2:%s/health\n' "$tcp_port"
run_host_probe M1 'slirp4netns:allow_host_loopback=true' "http://10.0.2.2:${tcp_port}/health"
m1_host=$host
m1_host_rc=$host_rc
run_internet_probe M1 'slirp4netns:allow_host_loopback=true'
m1_internet=$internet
m1_internet_rc=$internet_rc
printf 'GATE_LITERAL M1=false; candidate t3/argv.rs lines: replace --network=none with --network=slirp4netns:allow_host_loopback=true (1 line)\n'
printf 'T3_IMAGE M1=none; corpus-reauthor=network_escape_tcp_outbound_001,network_escape_udp_outbound_002,network_escape_dns_resolution_003,network_escape_ipv6_outbound_005,capability_escape_cap_net_raw_002\n'
notice "M1 host=${m1_host} internet=${m1_internet}" "$m1_host_rc/$m1_internet_rc"
if [ "$m1_host" != yes ] || [ "$m1_internet" != no ]; then
    contradiction "M1 expected-host=yes-internet=no" "$m1_host_rc/$m1_internet_rc"
fi

printf '+ M2 podman run --user=65534:65534 --read-only --network=pasta:--map-gw ... http://host.containers.internal:%s/health\n' "$tcp_port"
run_host_probe M2 'pasta:--map-gw' "http://host.containers.internal:${tcp_port}/health"
m2_host=$host
m2_host_rc=$host_rc
run_internet_probe M2 'pasta:--map-gw'
m2_internet=$internet
m2_internet_rc=$internet_rc
printf 'GATE_LITERAL M2=false; candidate t3/argv.rs lines: replace --network=none with --network=pasta:--map-gw (1 line)\n'
printf 'T3_IMAGE M2=passt/pasta availability required; corpus-reauthor=network_escape_tcp_outbound_001,network_escape_udp_outbound_002,network_escape_dns_resolution_003,network_escape_ipv6_outbound_005,capability_escape_cap_net_raw_002\n'
notice "M2 host=${m2_host} internet=${m2_internet}" "$m2_host_rc/$m2_internet_rc"
if [ "$m2_host" != yes ] || [ "$m2_internet" != no ]; then
    contradiction "M2 expected-host=yes-internet=no" "$m2_host_rc/$m2_internet_rc"
fi

printf '+ python3 host_unix_http.py %s 0600\n' "$unix_socket"
python3 "$egress_dir/host_unix_http.py" "$unix_socket" 0600 &
unix_pid=$!
for _ in $(seq 1 50); do
    if [ -S "$unix_socket" ]; then
        break
    fi
    sleep 0.1
done
if [ ! -S "$unix_socket" ]; then
    printf 'HOST_UNIX_START_FAILED\n'
    exit 70
fi
socket_mode=$(python3 -c 'import os, stat, sys; print(oct(stat.S_IMODE(os.stat(sys.argv[1]).st_mode)))' "$unix_socket")
printf 'M3_SOCKET_MODE_0600=%s custody=host-owner-only control under rootless userns\n' "$socket_mode"

run_m3() {
    if m3_output=$(podman run "${base[@]}" --network=none --volume="$unix_socket:/proxy/egress.sock:rw" "$image" -ec '
        /usr/bin/socat TCP-LISTEN:18080,bind=127.0.0.1,fork,reuseaddr UNIX-CONNECT:/proxy/egress.sock &
        forwarder=$!
        sleep 1
        lo=$(/sbin/ip link show lo)
        case "$lo" in
            *UP*) printf "M3_LO_UP=yes\\n" ;;
            *) printf "M3_LO_UP=no\\n" ;;
        esac
        if wget -qO- --timeout=5 http://127.0.0.1:18080/health; then
            printf "M3_HOST=yes\\n"
        else
            printf "M3_HOST=no\\n"
        fi
        if wget -qO- --timeout=5 https://example.com >/dev/null; then
            printf "M3_INTERNET=yes\\n"
        else
            printf "M3_INTERNET=no\\n"
        fi
        kill "$forwarder"
        if wait "$forwarder"; then
            printf "M3_FORWARDER_EXIT=0\n"
        else
            forwarder_rc=$?
            printf "M3_FORWARDER_EXIT=%s after intentional termination\n" "$forwarder_rc"
        fi
    '); then
        m3_rc=0
    else
        m3_rc=$?
    fi
    printf 'M3 rc=%s output=%s\n' "$m3_rc" "$m3_output"
}

printf '+ M3 control podman run --user=65534:65534 --read-only --network=none --volume=egress.sock:/proxy/egress.sock ...\n'
run_m3
case "$m3_output" in
    *M3_HOST=yes*) m3_0600_host=yes ;;
    *) m3_0600_host=no ;;
esac
printf 'M3_SOCKET_0600_HOST=%s rc=%s\n' "$m3_0600_host" "$m3_rc"
if [ "$m3_0600_host" = yes ]; then
    contradiction "M3 socket-0600 control unexpectedly connected" "$m3_rc"
fi

printf '+ chmod 0666 %s\n' "$unix_socket"
chmod 0666 "$unix_socket"
socket_mode=$(python3 -c 'import os, stat, sys; print(oct(stat.S_IMODE(os.stat(sys.argv[1]).st_mode)))' "$unix_socket")
printf 'M3_SOCKET_MODE_0666=%s custody=connectable by remapped uid; broader same-host-uid exposure than 0600\n' "$socket_mode"
printf '+ M3 podman run --user=65534:65534 --read-only --network=none --volume=egress.sock:/proxy/egress.sock ...\n'
run_m3
case "$m3_output" in
    *M3_HOST=yes*) m3_host=yes ;;
    *) m3_host=no ;;
esac
case "$m3_output" in
    *M3_INTERNET=no*) m3_internet=no ;;
    *) m3_internet=yes ;;
esac
case "$m3_output" in
    *M3_LO_UP=yes*) m3_lo=yes ;;
    *) m3_lo=no ;;
esac
m3_0666_host=$m3_host
printf 'M3_SOCKET_0666_HOST=%s rc=%s\n' "$m3_0666_host" "$m3_rc"
printf 'GATE_LITERAL M3=true; candidate t3/argv.rs lines: retain --network=none and add --volume=<egress.sock>:/proxy/egress.sock:rw (1 line)\n'
printf 'T3_IMAGE M3=socat forwarder + writable socket mount; update digest in t3-image.lock and re-sign the attestation; corpus unchanged because --network=none survives.\n'
notice "M3 host=${m3_host} internet=${m3_internet} lo=${m3_lo} socket-0600=${m3_0600_host} socket-0666=${m3_0666_host}" "$m3_rc"
if [ "$m3_host" != yes ] || [ "$m3_internet" != no ] || [ "$m3_lo" != yes ]; then
    contradiction "M3 expected-host=yes-internet=no-lo=yes" "$m3_rc"
fi

printf 'PRICE t3/argv.rs candidate kernel lines: M1=1 replacement; M2=1 replacement; M3=1 addition (overall 17-1 grant remains +65-130 kernel lines).\n'
if [ "$contradictions" -gt 0 ]; then
    exit 1
fi
