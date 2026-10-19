import type { IacFrame } from "maos:spirit/spirit@2.1.0";
import { recall, fetch } from "maos:spirit/recall@2.1.0";

import { halt } from "./halt.js";

let started = false;

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

  // Recall is emitter-only in this daemon run, not participant-scoped yet.
  recall({ limit: 16 });
  try {
    // The operator seed's emitter is pid 0, so fetching it is scope-violation.
    // Received frames become fetchable with
    // 17-8-participant-recall-and-consent-envelopes; success is normal too.
    fetch(frame.frameId);
  } catch (error) {
    // componentize-js wraps imported result errors in ComponentError.payload.
    if (
      typeof error !== "object" || error === null ||
      !("payload" in error) ||
      typeof error.payload !== "object" || error.payload === null ||
      !("tag" in error.payload) || error.payload.tag !== "scope-violation"
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
}
