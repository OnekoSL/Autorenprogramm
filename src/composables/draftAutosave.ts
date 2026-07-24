import { readonly, ref, type DeepReadonly, type Ref } from "vue";

export interface DraftAutosaveController<T> {
  readonly pending: DeepReadonly<Ref<boolean>>;
  readonly saving: DeepReadonly<Ref<boolean>>;
  schedule(draft: T): void;
  flush(): Promise<boolean>;
  reset(): void;
}

interface DraftAutosaveOptions<T> {
  delay: number;
  save: (draft: T) => Promise<boolean>;
}

export function createDraftAutosave<T>({
  delay,
  save,
}: DraftAutosaveOptions<T>): DraftAutosaveController<T> {
  const pending = ref(false);
  const saving = ref(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pendingDraft: T | undefined;
  let generation = 0;
  let running: Promise<{ generation: number; success: boolean }> | null = null;

  function clearTimer() {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  }

  async function run(runGeneration: number): Promise<boolean> {
    saving.value = true;
    try {
      while (runGeneration === generation && pendingDraft !== undefined) {
        const draft = pendingDraft;
        pendingDraft = undefined;
        pending.value = false;

        if (!await save(draft)) {
          if (runGeneration === generation && pendingDraft === undefined) {
            pendingDraft = draft;
          }
          if (runGeneration === generation) {
            pending.value = true;
          }
          return false;
        }
      }
      return true;
    } finally {
      saving.value = false;
    }
  }

  async function flush(): Promise<boolean> {
    clearTimer();

    if (running) {
      const result = await running;
      if (pendingDraft !== undefined && (result.success || result.generation !== generation)) {
        return flush();
      }
      return result.success;
    }

    if (pendingDraft === undefined) {
      return true;
    }

    const runGeneration = generation;
    const runPromise = run(runGeneration).then((success) => ({ generation: runGeneration, success }));
    running = runPromise;
    const result = await runPromise;
    if (running === runPromise) {
      running = null;
    }

    if (pendingDraft !== undefined && (result.success || result.generation !== generation)) {
      return flush();
    }
    return result.success;
  }

  function schedule(draft: T) {
    pendingDraft = draft;
    pending.value = true;
    clearTimer();
    const scheduledGeneration = generation;
    timer = setTimeout(() => {
      timer = undefined;
      if (scheduledGeneration === generation) {
        void flush();
      }
    }, delay);
  }

  function reset() {
    clearTimer();
    generation += 1;
    pendingDraft = undefined;
    pending.value = false;
  }

  return {
    pending: readonly(pending),
    saving: readonly(saving),
    schedule,
    flush,
    reset,
  };
}
