<script setup lang="ts">
import ProjectStartView from "./components/ProjectStartView.vue";
import WorkspaceShell from "./components/WorkspaceShell.vue";
import { useAppBootstrap } from "./composables/useAppBootstrap";
import { useProjectStore } from "./stores/project";

const projectStore = useProjectStore();
const { appInfo, appInfoFailed } = useAppBootstrap();
</script>

<template>
  <ProjectStartView v-if="!projectStore.activeProject" />
  <WorkspaceShell
    v-else
    :app-info="appInfo"
    :app-info-failed="appInfoFailed"
  />
</template>
