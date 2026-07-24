<script lang="ts">
import { defineComponent } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";

export default defineComponent({
  name: "WorkspaceStatusBar",
  setup() {
    return useWorkspaceViewContext();
  },
});
</script>

<template>
<footer class="statusbar">
      <template v-if="workspace.activeSection === 'structure'">
        <span>{{ storyStructureStore.displayedStructure?.name || ui("Struktur") }}</span>
        <span>{{ t("counts.acts", storyStructureStore.displayedActs.length) }}</span>
        <span>{{ t("counts.beats", structureBeatCount) }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'bible'">
        <span>{{ t("bible.cards") }}: {{ bibleSaveState }}</span>
        <span>{{ projectBibleStore.cards.length }} {{ selectedCardTypeLabel.toLocaleLowerCase(localeCode()) }}</span>
        <span>{{ t("counts.linkedScenes", projectBibleStore.selectedCardLinks.length) }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'worldbuilding'">
        <span>{{ t("status.worldSystem", { state: worldbuildingSaveState.toLocaleLowerCase(localeCode()) }) }}</span>
        <span>{{ t("counts.systems", worldbuildingStore.systems.length) }}</span>
        <span v-if="worldbuildingStore.selectedSystem">{{ worldSystemDraft.name || t("dynamic.unnamed") }} · {{ worldCategoryLabel(worldSystemDraft.category) }}</span>
        <span v-else>{{ ui("Kein Weltsystem ausgewählt") }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'settings'">
        <span>{{ ui("Projekteinstellungen") }}</span>
        <span>{{ projectSettingsDraft.defaultLanguage }}</span>
        <span>{{ projectStore.activeProject?.path }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'search'">
        <span>{{ searchStore.query.trim() ? t("counts.results", searchStore.results.length) : t("common.search") }}</span>
        <span>{{ t("counts.scenes", manuscriptStore.scenes.length) }}</span>
        <span>{{ t(searchStore.isLoading ? "dynamic.running" : "dynamic.ready") }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'export'">
        <span>{{ t(exportStore.isExporting ? "dynamic.exportRunning" : "dynamic.exportReady") }}</span>
        <span>{{ t("counts.areas", manuscriptStore.areas.length) }}</span>
        <span>{{ exportStore.lastExportPath || "Markdown / DOCX" }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'ai'">
        <span>{{ ui("KI-Chat") }}</span>
        <span>{{ t("counts.messages", aiStore.chatMessages.length) }}</span>
        <span>{{ aiStore.providerStatus ? t(`provider.${aiStore.providerStatus.code}`) : t("dynamic.unknown") }}</span>
      </template>
      <template v-else>
        <span>{{ t("status.shortcut", { state: saveState.toLocaleLowerCase(localeCode()) }) }}</span>
        <span>{{ t("counts.words", manuscriptStore.wordCount) }}</span>
        <span>{{ manuscriptStore.activeLocation }}</span>
      </template>
      <span v-if="appInfo">{{ appInfo.name }} {{ appInfo.version }} / {{ appInfo.environment }}</span>
      <span v-else-if="appInfoFailed">{{ ui("Backend nicht verbunden") }}</span>
      <button type="button" @click="workspace.resetLayout()">{{ ui("Layout zurücksetzen") }}</button>
    </footer>
</template>
