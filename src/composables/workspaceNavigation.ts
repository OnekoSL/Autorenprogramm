import { inject, provide, type InjectionKey } from "vue";
import type { WorkspaceSection } from "../stores/workspace";

export interface WorkspaceViewGuard {
  beforeLeave(): Promise<boolean>;
}

export interface WorkspaceNavigation {
  register(section: WorkspaceSection, guard: WorkspaceViewGuard["beforeLeave"]): void;
  unregister(section: WorkspaceSection, guard?: WorkspaceViewGuard["beforeLeave"]): void;
  beforeLeave(section: WorkspaceSection): Promise<boolean>;
}

const workspaceNavigationKey: InjectionKey<WorkspaceNavigation> = Symbol("workspace-navigation");

export function createWorkspaceNavigation(): WorkspaceNavigation {
  const guards = new Map<WorkspaceSection, WorkspaceViewGuard["beforeLeave"]>();
  return {
    register(section, guard) {
      guards.set(section, guard);
    },
    unregister(section, guard) {
      if (!guard || guards.get(section) === guard) guards.delete(section);
    },
    async beforeLeave(section) {
      return await guards.get(section)?.() ?? true;
    },
  };
}

export function provideWorkspaceNavigation(): WorkspaceNavigation {
  const navigation = createWorkspaceNavigation();
  provide(workspaceNavigationKey, navigation);
  return navigation;
}

export function useWorkspaceNavigation(): WorkspaceNavigation {
  const navigation = inject(workspaceNavigationKey);
  if (!navigation) throw new Error("workspace_navigation_unavailable");
  return navigation;
}
