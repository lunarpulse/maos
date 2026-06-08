# Story 8.13 Tier-2a Release Gate — Real Mobile Push

Status: OPEN

## Claim Under Test

A real generic-HTTP mobile push reached a real phone using the same `maos-notify-push` operator-config seam shipped in Story 8.13.

## Required Evidence

- Provider: TBD by deployer/operator (ntfy, Pushover message API, Gotify, webhook-compatible relay, or equivalent generic POST target)
- Endpoint: TBD, redacted before recording
- Token: REDACTED; never paste token or full bearer value
- Command: `MAOS_ONE_SHOT=smoke-mira-nash-tcp-8-13 ./target/debug/maos-bin` with operator config pointed at the real provider
- Observed phone receipt: TBD by named human operator physically holding the phone
- Transport co-sign: TBD by Winston after confirming the same generic HTTP POST adapter and bounded-timeout path were used

## Sign-Off

- Human operator: OPEN
- Winston transport co-sign: OPEN

## Notes

Tier-2b, the two-OS-process A2A run, is explicitly deferred to the follow-up story for standalone-loadable Mira/Nash plus daemon A2A composition-root wiring. It is not a Story 8.13 release gate.
