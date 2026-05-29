import { describe, it, expect } from "vitest";
import { SpiritTest, MockCtx } from "../src/spirit_test/index.js";
import { NeverCancel } from "../src/spirit.js";

describe("SDK smoke", () => {
  it("SpiritTest harness wires lifecycle correctly", () => {
    const mockSpirit = {
      onIdle(ctx: { cancellation(): { isCancelled(): boolean } }) {
        expect(ctx.cancellation().isCancelled()).toBe(false);
      },
    };
    const harness = new SpiritTest(mockSpirit);
    const report = harness.run();
    expect(report.hooksFired.get("on_idle")).toBe(1);
  });

  it("MockCtx exposes deprecation warnings", () => {
    const ctx = new MockCtx([
      { surface: "Test::old", sinceVersion: "0.5", plannedRemoval: "1.0", migrationHint: "use Test::new" },
    ]);
    expect(ctx.deprecationWarnings()).toHaveLength(1);
  });
});
