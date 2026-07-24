import { describe, expect, it, vi } from "vitest";
import { createWorkspaceNavigation } from "./workspaceNavigation";

describe("workspace navigation", () => {
  it("allows sections without a registered guard", async () => {
    const navigation = createWorkspaceNavigation();

    await expect(navigation.beforeLeave("search")).resolves.toBe(true);
  });

  it("waits for the active view guard and propagates a failed flush", async () => {
    const navigation = createWorkspaceNavigation();
    const beforeLeave = vi.fn().mockResolvedValue(false);
    navigation.register("writing", beforeLeave);

    await expect(navigation.beforeLeave("writing")).resolves.toBe(false);
    expect(beforeLeave).toHaveBeenCalledOnce();
  });

  it("only unregisters the guard that is still active", async () => {
    const navigation = createWorkspaceNavigation();
    const first = vi.fn().mockResolvedValue(false);
    const second = vi.fn().mockResolvedValue(true);
    navigation.register("bible", first);
    navigation.register("bible", second);

    navigation.unregister("bible", first);
    await expect(navigation.beforeLeave("bible")).resolves.toBe(true);
    expect(second).toHaveBeenCalledOnce();

    navigation.unregister("bible", second);
    await expect(navigation.beforeLeave("bible")).resolves.toBe(true);
  });
});
