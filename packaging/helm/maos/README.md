# maos Helm chart

Deploys one standing MAOS Host (`maos serve`, ADR-071) as a one-replica StatefulSet with a
persistent home at `/var/lib/maos`. Helm 3, Kubernetes >= 1.27.

```sh
helm install maos packaging/helm/maos \
  --set image.repository=ghcr.io/lunarpulse/maos --set image.tag=<tag>
kubectl exec maos-0 -c maos -- /usr/local/bin/maosctl status
```

- **Host file.** `.Values.hostToml` is rendered into a ConfigMap and mounted read-only at
  `/etc/maos/host.toml`. The default cast is the in-tree Spirit manifests baked into the image
  (nash, observer), which needs no provider key.
- **Secrets.** `.Values.secrets` (rendered into a Secret) or `.Values.existingSecret` are injected
  as environment variables; every name must be a registered `MAOS_*` name.
- **Init.** An init container runs `maos init --plain`, the one minter of the operator door config.
- **Probes.** Readiness and liveness are `exec: /usr/local/bin/maosctl status`. The door is
  loopback-only (a `door_bind` that is not loopback is a typed refusal), so an `httpGet` probe
  cannot reach it, and the chart renders no Service and no `containerPort` for it.
- **Security context.** `runAsNonRoot`, uid/gid/fsGroup 65532, read-only root filesystem, no
  privilege escalation, all capabilities dropped. `HOME=/var/lib/maos` puts every fallback data
  root on the volume.
- **Shutdown.** `terminationGracePeriodSeconds: 30`, matching the systemd unit's `TimeoutStopSec`.

## T3 Workers are not supported in a pod

T3 Workers need a rootless container runtime (the detector probes `/usr/bin/podman`), which a pod
does not provide by default. This chart mounts no container socket and requests no privilege to
work around that.
