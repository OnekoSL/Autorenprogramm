<script setup lang="ts">
import { computed, toRef, type Component } from "vue";
import type { AppInfo } from "../api/app";
import { useWorkspaceController } from "../composables/useWorkspaceController";
import { provideWorkspaceNavigation } from "../composables/workspaceNavigation";
import { provideWorkspaceViewContext } from "../composables/workspaceViewContext";
import type { WorkspaceSection } from "../stores/workspace";
import AiView from "../views/AiView.vue";
import ExportView from "../views/ExportView.vue";
import ProjectBibleView from "../views/ProjectBibleView.vue";
import SearchView from "../views/SearchView.vue";
import SettingsView from "../views/SettingsView.vue";
import StructureView from "../views/StructureView.vue";
import WorldbuildingView from "../views/WorldbuildingView.vue";
import WritingView from "../views/WritingView.vue";
import WorkspaceHeader from "./WorkspaceHeader.vue";
import WorkspaceNavigatorPanel from "./WorkspaceNavigatorPanel.vue";
import WorkspaceSidebarPanel from "./WorkspaceSidebarPanel.vue";
import WorkspaceStatusBar from "./WorkspaceStatusBar.vue";

const props = defineProps<{
  appInfo: AppInfo | null;
  appInfoFailed: boolean;
}>();

const navigation = provideWorkspaceNavigation();
const controller = useWorkspaceController(navigation);
const viewGuards = {
  writing: controller.persistSceneDraftNow,
  bible: controller.persistCardDraftNow,
  structure: controller.flushBeatSave,
  worldbuilding: controller.persistWorldSystemDraftNow,
};

provideWorkspaceViewContext({
  ...controller,
  appInfo: toRef(props, "appInfo"),
  appInfoFailed: toRef(props, "appInfoFailed"),
  viewGuards,
});

const workspaceViews: Record<WorkspaceSection, Component> = {
  writing: WritingView,
  bible: ProjectBibleView,
  structure: StructureView,
  worldbuilding: WorldbuildingView,
  search: SearchView,
  export: ExportView,
  ai: AiView,
  settings: SettingsView,
};

const activeWorkspaceView = computed(() => workspaceViews[controller.workspace.activeSection]);
const workspaceStyle = controller.workspaceStyle;
</script>

<template>
  <main class="app-shell">
    <WorkspaceHeader />

    <section
      class="workspace"
      :style="workspaceStyle"
      :aria-label="controller.ui('Schreibarbeitsplatz')"
    >
      <WorkspaceNavigatorPanel />
      <KeepAlive>
        <component
          :is="activeWorkspaceView"
          :key="controller.workspace.activeSection"
        />
      </KeepAlive>
      <WorkspaceSidebarPanel />
    </section>

    <WorkspaceStatusBar />
  </main>
</template>
