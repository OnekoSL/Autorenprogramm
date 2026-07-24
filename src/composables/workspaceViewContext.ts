import { inject, provide, type InjectionKey, type Ref } from "vue";
import type { AppInfo } from "../api/app";
import type { WorkspaceSection } from "../stores/workspace";
import type { useWorkspaceController } from "./useWorkspaceController";
import type { WorkspaceViewGuard } from "./workspaceNavigation";

export type WorkspaceViewContext = ReturnType<typeof useWorkspaceController> & {
  appInfo: Readonly<Ref<AppInfo | null>>;
  appInfoFailed: Readonly<Ref<boolean>>;
  viewGuards: Partial<Record<WorkspaceSection, WorkspaceViewGuard["beforeLeave"]>>;
};

const workspaceViewContextKey: InjectionKey<WorkspaceViewContext> = Symbol("workspace-view-context");

export function provideWorkspaceViewContext(context: WorkspaceViewContext): void {
  provide(workspaceViewContextKey, context);
}

export function useWorkspaceViewContext(): WorkspaceViewContext {
  const context = inject(workspaceViewContextKey);
  if (!context) throw new Error("workspace_view_context_unavailable");
  return context;
}
