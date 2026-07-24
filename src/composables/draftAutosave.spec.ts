import { describe, expect, it, vi } from "vitest";
import { createDraftAutosave } from "./draftAutosave";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

describe("createDraftAutosave", () => {
  it("debounct mehrere Änderungen auf den jüngsten Entwurf", async () => {
    vi.useFakeTimers();
    const save = vi.fn<(draft: { value: number }) => Promise<boolean>>().mockResolvedValue(true);
    const controller = createDraftAutosave({ delay: 400, save });

    controller.schedule({ value: 1 });
    controller.schedule({ value: 2 });
    controller.schedule({ value: 3 });

    await vi.advanceTimersByTimeAsync(399);
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledOnce();
    expect(save).toHaveBeenCalledWith({ value: 3 });
    expect(controller.pending.value).toBe(false);
    vi.useRealTimers();
  });

  it("serialisiert eine neue Revision während einer laufenden Speicherung", async () => {
    const firstSave = deferred<boolean>();
    const save = vi.fn<(draft: { value: number }) => Promise<boolean>>()
      .mockImplementationOnce(() => firstSave.promise)
      .mockResolvedValueOnce(true);
    const controller = createDraftAutosave({ delay: 400, save });

    controller.schedule({ value: 1 });
    const flush = controller.flush();
    await Promise.resolve();
    controller.schedule({ value: 2 });
    firstSave.resolve(true);

    await expect(flush).resolves.toBe(true);
    expect(save.mock.calls).toEqual([[{ value: 1 }], [{ value: 2 }]]);
  });

  it("speichert vor einem Auswahlwechsel und blockiert ihn bei einem Fehler", async () => {
    const order: string[] = [];
    const save = vi.fn(async () => {
      order.push("save");
      return false;
    });
    const controller = createDraftAutosave({ delay: 400, save });
    let selected = "scene-a";
    controller.schedule({ id: "scene-a", text: "Neue Fassung" });

    if (await controller.flush()) {
      selected = "scene-b";
      order.push("select");
    }

    expect(selected).toBe("scene-a");
    expect(order).toEqual(["save"]);
    expect(controller.pending.value).toBe(true);
  });

  it("behält einen fehlgeschlagenen Entwurf für einen erneuten Flush", async () => {
    const save = vi.fn<(draft: { value: number }) => Promise<boolean>>()
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);
    const controller = createDraftAutosave({ delay: 400, save });
    controller.schedule({ value: 7 });

    await expect(controller.flush()).resolves.toBe(false);
    await expect(controller.flush()).resolves.toBe(true);
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ value: 7 });
  });

  it("verwirft beim Reset alte Revisionen und speichert eine neue Generation", async () => {
    const firstSave = deferred<boolean>();
    const save = vi.fn<(draft: { value: number }) => Promise<boolean>>()
      .mockImplementationOnce(() => firstSave.promise)
      .mockResolvedValueOnce(true);
    const controller = createDraftAutosave({ delay: 400, save });
    controller.schedule({ value: 1 });
    const oldFlush = controller.flush();
    await Promise.resolve();

    controller.reset();
    controller.schedule({ value: 2 });
    const newFlush = controller.flush();
    firstSave.resolve(true);

    await expect(oldFlush).resolves.toBe(true);
    await expect(newFlush).resolves.toBe(true);
    expect(save.mock.calls).toEqual([[{ value: 1 }], [{ value: 2 }]]);
  });
});
