<script lang="ts">
import { defineComponent } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";

export default defineComponent({
  name: "WorkspaceHeader",
  setup() {
    return useWorkspaceViewContext();
  },
});
</script>

<template>
<header class="topbar">
      <div>
        <p class="eyebrow">Autorenprogramm</p>
        <h1>{{ projectStore.activeProject?.name }}</h1>
      </div>

      <div class="topbar-actions" :aria-label="ui('Globale Aktionen')">
        <button type="button" :disabled="!canUndoSceneText" @click="undoSceneText">
          {{ ui("Rückgängig") }}
        </button>
        <button type="button" :disabled="!canRedoSceneText" @click="redoSceneText">
          {{ ui("Wiederholen") }}
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'writing' && workspace.sidebarTab !== 'ai' }"
          @click="openTopbarWriting"
        >
          {{ ui("Schreiben") }}
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'bible' }"
          @click="openTopbarBible"
        >
          {{ ui("Projektbibel") }}
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'structure' }"
          @click="openTopbarSection('structure')"
        >
          {{ ui("Struktur") }}
        </button>
        <button type="button" :class="{ active: workspace.activeSection === 'worldbuilding' }" @click="openTopbarSection('worldbuilding')">{{ ui("Worldbuilding") }}</button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'search' }"
          @click="openTopbarSection('search')"
        >
          {{ ui("Suche") }}
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'export' }"
          @click="openTopbarSection('export')"
        >
          {{ ui("Export") }}
        </button>
        <button type="button" :class="{ active: workspace.activeSection === 'ai' }" @click="openTopbarAi">
          {{ ui("KI") }}
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'settings' }"
          @click="openTopbarSection('settings')"
        >
          {{ ui("Setup") }}
        </button>
        <button type="button" :disabled="projectStore.isSavingCopy" @click="saveProjectCopyAs">
          {{ ui("Speichern unter") }}
        </button>
        <span class="save-state" :class="{ problem: manuscriptStore.error }">{{ saveState }}</span>
        <span
          v-if="projectCopyState"
          class="save-state"
          :class="{ problem: projectStore.copyError }"
        >
          {{ projectCopyState }}
        </span>
      </div>
    </header>
</template>
