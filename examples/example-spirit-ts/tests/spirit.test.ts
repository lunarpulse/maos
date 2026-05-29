import { describe, it, expect } from "vitest";
import { SpiritTest, assert } from "@maos/spirit-ts/spirit_test";
import { ExampleTsSpirit } from "../src/index";

describe("ExampleTsSpirit smoke", () => {
  it("on_idle fires without error", () => {
    const spirit = new ExampleTsSpirit();
    const harness = new SpiritTest(spirit);
    const report = harness.run();
    assert(report.hooksFired.get("on_idle") === 1, "on_idle did not fire exactly once");
  });
});
