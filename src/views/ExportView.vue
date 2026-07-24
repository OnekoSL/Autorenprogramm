<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "ExportView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["export"];
    if (guard) navigation.register("export", guard);
    onUnmounted(() => navigation.unregister("export", guard));
    return context;
  },
});
</script>

<template>
<section
        class="editor utility-editor"
        :aria-label="ui('Export')"
      >
        <div class="utility-panel">
          <p class="eyebrow">{{ ui("Export") }}</p>
          <h2>{{ ui("Manuskript exportieren") }}</h2>
          <fieldset class="export-content-options" :disabled="exportStore.isLoadingProfile || exportStore.isSavingProfile">
            <legend>{{ ui("Inhalt auswählen") }}</legend>
            <label>
              <input type="checkbox" :checked="exportStore.profile?.includeNovelTitle" @change="updateExportOption('includeNovelTitle', $event)" />
              <span>{{ ui("Romantitel") }}</span>
            </label>
            <label>
              <input type="checkbox" :checked="exportStore.profile?.includeAreaTitles" @change="updateExportOption('includeAreaTitles', $event)" />
              <span>{{ ui("Bereichstitel") }}</span>
            </label>
            <label>
              <input type="checkbox" :checked="exportStore.profile?.includeChapterTitles" @change="updateExportOption('includeChapterTitles', $event)" />
              <span>{{ ui("Kapiteltitel") }}</span>
            </label>
            <label>
              <input type="checkbox" :checked="exportStore.profile?.includeSceneTitles" @change="updateExportOption('includeSceneTitles', $event)" />
              <span>{{ ui("Szenentitel") }}</span>
            </label>
            <label>
              <input type="checkbox" :checked="exportStore.profile?.includeSceneText" @change="updateExportOption('includeSceneText', $event)" />
              <span>{{ ui("Szenentext") }}</span>
            </label>
          </fieldset>
          <p class="export-profile-summary">{{ t("dynamic.selected") }}: {{ exportStore.profileSummary || t("dynamic.nothing") }}</p>
          <div class="utility-option-grid">
            <button
              type="button"
              :disabled="!exportStore.canExport"
              @click="exportManuscript('markdown')"
            >
              {{ ui("Markdown exportieren") }}
            </button>
            <button
              type="button"
              :disabled="!exportStore.canExport"
              @click="exportManuscript('docx')"
            >
              {{ ui("DOCX exportieren") }}
            </button>
          </div>
          <p v-if="exportStore.error" class="error-text">{{ formatUiError(exportStore.error) }}</p>
          <p v-else-if="exportStore.isExporting" class="muted-text">{{ ui("Export wird geschrieben...") }}</p>
          <p v-else-if="exportStore.lastExportPath" class="muted-text">
            {{ t("exportProfile.exported", { path: exportStore.lastExportPath }) }}
          </p>
          <p class="muted-text">{{ ui("Die Auswahl wird automatisch im Projekt gespeichert und gilt für Markdown und DOCX. Kurzbeschreibungen, Projektbibel und KI-Vorschläge bleiben intern.") }}</p>
        </div>
      </section>
</template>
