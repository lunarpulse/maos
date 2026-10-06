import type { IacFrame } from "maos:spirit/spirit@2.0.0";

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

  return [frame];
}

export function onStart(): void {
  started = true;
}

export function onShutdown(): void {
  started = false;
}
