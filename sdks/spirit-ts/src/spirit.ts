// Spirit interface — 14 hooks mirror of the Rust Spirit trait.
// At v0.5 only onIdle is scaffolded; authors add more hooks as needed.

export interface Ctx {
  cancellation(): CancellationSignal;
  deprecationWarnings(): DeprecationWarning[];
}

export interface CancellationSignal {
  isCancelled(): boolean;
}

export interface DeprecationWarning {
  surface: string;
  sinceVersion: string;
  plannedRemoval: string;
  migrationHint: string;
}

export interface Spirit {
  onIdle(ctx: Ctx): void;
}

// Mock bus for test harness — forward-anchor for frame capture.
export interface MockBus {
  send(bytes: Uint8Array): void;
}

export class NeverCancel implements CancellationSignal {
  isCancelled(): boolean {
    return false;
  }
}
