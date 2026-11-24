#!/usr/bin/env bash
# Story 22-3 per-PR proof (discipline.yml `container-build`): `helm install` on a pinned kind
# cluster runs the Host, the exec probe passes, and a pod delete keeps the cast from the PVC.
# Needs kind, helm 3, kubectl and MAOS_IMAGE (a local image, e.g. maos:ci).
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${MAOS_IMAGE:?set MAOS_IMAGE}"
repo=${MAOS_IMAGE%:*}
tag=${MAOS_IMAGE##*:}
cluster=maos-ci
node_image='kindest/node:v1.31.2@sha256:18fbefc20a7113353c7b75b5c869d7145a6abd6269154825872dc59c1329912e'
trap 'kubectl logs maos-0 -c maos --tail=100 || true; kind delete cluster --name "$cluster" || true' EXIT

helm lint packaging/helm/maos
helm template maos packaging/helm/maos >/dev/null

kind create cluster --name "$cluster" --image "$node_image" --wait 180s
kind load docker-image "$MAOS_IMAGE" --name "$cluster"
helm install maos packaging/helm/maos \
  --set image.repository="$repo" --set image.tag="$tag" --set image.pullPolicy=Never \
  --wait --timeout 300s

cast() { kubectl exec maos-0 -c maos -- /usr/local/bin/maosctl agents | grep -oE 'nash|observer' | sort -u; }
want=$'nash\nobserver'
wait_ready() { kubectl wait --for=condition=ready pod/maos-0 --timeout=300s; }

kubectl exec maos-0 -c maos -- /usr/local/bin/maosctl status
[ "$(cast)" = "$want" ] || { echo "cast after install is not nash+observer" >&2; exit 1; }
if kubectl logs maos-0 -c init | grep -i 'could not create'; then
  echo "maos init could not create a directory under the read-only root" >&2
  exit 1
fi
# The door never leaves loopback: no Service and no declared container port.
[ -z "$(kubectl get svc -o name)" ] || [ "$(kubectl get svc -o name)" = service/kubernetes ] \
  || { echo "the chart rendered a Service" >&2; exit 1; }
[ -z "$(kubectl get pod maos-0 -o jsonpath='{.spec.containers[*].ports}')" ] \
  || { echo "the pod declares a container port" >&2; exit 1; }

# A pod delete keeps the cast: the home is on the PVC and the Host file names the cast.
kubectl delete pod maos-0 --wait=true
for _ in $(seq 60); do kubectl get pod maos-0 >/dev/null 2>&1 && break; sleep 2; done
wait_ready
kubectl exec maos-0 -c maos -- /usr/local/bin/maosctl status
[ "$(cast)" = "$want" ] || { echo "cast after pod delete differs" >&2; exit 1; }
echo "helm proof passed"
