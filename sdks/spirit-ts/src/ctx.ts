// Ctx mock implementation + deprecation_warnings surface.

import { Ctx, CancellationSignal, DeprecationWarning, NeverCancel } from "./spirit.js";

export class MockCtx implements Ctx {
  private cancel: CancellationSignal;
  private warnings: DeprecationWarning[];

  constructor(warnings: DeprecationWarning[] = []) {
    this.cancel = new NeverCancel();
    this.warnings = warnings;
  }

  cancellation(): CancellationSignal {
    return this.cancel;
  }

  deprecationWarnings(): DeprecationWarning[] {
    return this.warnings;
  }
}
