import type { IacFrame } from "maos:spirit/spirit-lifecycle@2.4.0";
import { recall, fetch } from "maos:spirit/recall@2.4.0";
import { author } from "maos:spirit/skills@2.4.0";

import { halt } from "./halt.js";

let started = false;
let authored = false;

/**
 * Echoes one inbound ADR-032 frame after the guest has started.
 *
 * `jco guest-types` exposes this as `IacFrame[]` and omits WIT's
 * `result<..., halt>` wrapper. Throwing `{ tag: "fault", val: reason }` maps
 * to `Halt::Fault` (R17-39c). `console.log` output is journaled by the host as
 * bounded, untrusted `spirit.diagnostic` rows (R17-39b).
 */
export function handleFrame(frame: IacFrame): IacFrame[] {
  if (!started) {
    halt("handleFrame called before onStart");
  }

  if (!authored) {
    try {
      author(`---
id='example-spirit-ts.example'
version='1.0.0'
name='Example skill'
description='A skill proposed for operator admission'
required_capabilities=['skill.author.self']
---
Follow the example procedure.
`);
    } catch (error) {
      if (
        typeof error !== "object" || error === null || !("payload" in error) ||
        typeof error.payload !== "object" || error.payload === null ||
        !("tag" in error.payload) ||
        (error.payload.tag !== "refused" && error.payload.tag !== "unavailable" &&
          error.payload.tag !== "capability-denied")
      ) {
        throw error;
      }
      console.log("Skill authoring declined:", error.payload.tag,
        "val" in error.payload ? error.payload.val : "");
    }
    authored = true;
  }

  // Recall lists frames this Spirit sent or received in this daemon run.
  // Withheld frames are omitted; payload-available = false means header-only.
  recall({ limit: 16 });
  try {
    // The --once operator seed is received by this Spirit, so fetch succeeds.
    // consent-withheld means the granter's consent no longer permits payload
    // disclosure (including header-only recall); scope-violation is normal too.
    fetch(frame.frameId);
  } catch (error) {
    // componentize-js wraps imported result errors in ComponentError.payload.
    if (
      typeof error !== "object" || error === null ||
      !("payload" in error) ||
      typeof error.payload !== "object" || error.payload === null ||
      !("tag" in error.payload) ||
      (error.payload.tag !== "scope-violation" &&
        error.payload.tag !== "consent-withheld")
    ) {
      throw error;
    }
  }

  return [frame];
}

export function onStart(): void {
  started = true;
}

export function onShutdown(): void {
  started = false;
  authored = false;
}

let idles = 0;

const text = new TextEncoder();
const plain = new TextDecoder();

/**
 * The optional `maos:spirit/lifecycle@2.4.0` export (Story 20-9). The kernel calls one
 * export at a time, under the manifest's `[budget].time_cap_seconds`, and never during a
 * `handleFrame` turn. A Spirit without this export keeps working: it is simply never called.
 *
 * Every export returns `result<_, lifecycle-error>`; `jco` omits the wrapper, so success
 * returns the value and a refusal throws `{ tag: "declined" | "failed", val: reason }`.
 * None of them emits frames. `idle`, `consolidate`, `telemetry` and `schedule` may call
 * `recall`, `context` and `skills`; `pause`, `resume`, `snapshot`, `swap-in` and `migrate`
 * are import-free, so a hot swap never depends on the log.
 */
export const lifecycle = {
  idle(): void {
    idles += 1;
  },
  consolidate(_batch: Uint8Array): void {},
  telemetry(_event: unknown): void {},
  schedule(_payload: Uint8Array): void {},
  pause(): void {},
  resume(): void {},
  /** The state this Spirit chooses to carry across a hot swap (at a turn boundary). */
  snapshot(): Uint8Array {
    return text.encode(String(idles));
  },
  /** The successor continues from the predecessor's snapshot (after `migrate`, if any). */
  swapIn(state: Uint8Array): void {
    const carried = Number(plain.decode(state));
    idles = Number.isFinite(carried) ? carried : 0;
  },
  /** `--policy migrator`: turn a predecessor's snapshot into this version's state. */
  migrate(state: Uint8Array): Uint8Array {
    return state;
  },
};
