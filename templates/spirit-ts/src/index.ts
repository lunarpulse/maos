// {{class_name}} — a MAOS Spirit scaffolded from templates/spirit-ts.
//
// Edit `onIdle` to implement your Spirit's idle-time behavior. See
// README.md for the 30-minute first-Spirit path.

import { Spirit, Ctx } from "@maos/spirit-ts";

export class {{class_name}} implements Spirit {
  onIdle(ctx: Ctx): void {
    // Bail early if the kernel has signaled cancellation.
    if (ctx.cancellation().isCancelled()) {
      return;
    }
    // TODO: implement your Spirit's idle behavior here.
  }
}
