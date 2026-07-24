<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "SettingsView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["settings"];
    if (guard) navigation.register("settings", guard);
    onUnmounted(() => navigation.unregister("settings", guard));
    return context;
  },
});
</script>

<template>
<section
        class="editor settings-editor"
        :aria-label="ui('Projekteinstellungen')"
      >
        <form class="settings-form" @submit.prevent="saveProjectSettings">
          <div class="settings-heading">
            <div>
              <p class="eyebrow">{{ ui("Setup") }}</p>
              <h2>{{ ui("Projekteinstellungen") }}</h2>
            </div>
            <button type="submit" class="primary-action" :disabled="!canSaveProjectSettings">
              {{ ui("Projekteinstellungen speichern") }}
            </button>
          </div>

          <label>
            <span>{{ ui("Projektname") }}</span>
            <input v-model="projectSettingsDraft.name" autocomplete="off" />
          </label>

          <label>
            <span>{{ ui("Beschreibung") }}</span>
            <textarea v-model="projectSettingsDraft.description" />
          </label>

          <label>
            <span>{{ t("project.language") }}</span>
            <input :value="projectSettingsDraft.defaultLanguage === 'en' ? t('project.english') : t('project.german')" readonly />
            <small>{{ t("project.fixedLanguage") }}</small>
          </label>

          <section class="settings-subsection" :aria-label="ui('Editor')">
            <h3>{{ ui("Editor") }}</h3>
            <label>
              <span>{{ ui("Schriftart") }}</span>
              <select v-model="projectSettingsDraft.editorFontFamily">
                <option v-for="font in editorFontOptions" :key="font.id" :value="font.id">
                  {{ font.label }}
                </option>
              </select>
            </label>

            <label>
              <span>{{ ui("Schriftgröße") }}</span>
              <select v-model.number="projectSettingsDraft.editorFontSize">
                <option v-for="fontSize in editorFontSizeOptions" :key="fontSize" :value="fontSize">
                  {{ fontSize }} px
                </option>
              </select>
            </label>
          </section>

          <p v-if="projectStore.error" class="error-text">{{ formatUiError(projectStore.error) }}</p>
        </form>

        <section class="settings-info project-defaults-panel" :aria-label="ui('Vorgaben für neue Projekte')">
          <div class="settings-heading">
            <div>
              <p class="eyebrow">{{ ui("Neue Projekte") }}</p>
              <h3>{{ ui("Standardstruktur") }}</h3>
            </div>
            <button
              type="button"
              class="primary-action"
              :disabled="projectStore.isSavingDefaults || !projectDefaultsTemplateId"
              @click="saveProjectDefaults"
            >
              {{ ui("Standard speichern") }}
            </button>
          </div>
          <label>
            <span>{{ ui("Strukturvorlage für neue Projekte") }}</span>
            <select v-model="projectDefaultsTemplateId" :disabled="projectStore.isSavingDefaults">
              <option
                v-for="template in storyStructureStore.templates"
                :key="template.id"
                :value="template.id"
              >
                {{ template.name }}
              </option>
            </select>
          </label>
          <p class="muted-text">
            {{ ui("Gilt nur für neue Projekte. Die Struktur dieses Projekts wird nicht geändert.") }}
          </p>
          <p v-if="projectStore.defaultsError" class="error-text">{{ formatUiError(projectStore.defaultsError) }}</p>
        </section>

        <section class="settings-info ai-settings-panel" :aria-label="ui('Lokale KI')">
          <h3>{{ ui("Lokale KI") }}</h3>
          <div class="ai-provider-status">
            <label v-if="aiStore.hasModels" class="ai-model-picker">
              <span>{{ ui("Modell") }}</span>
              <select
                :value="aiStore.settings?.model"
                :disabled="aiStore.isSaving"
                @change="selectAiModel"
              >
                <option value="" disabled>{{ ui("Modell auswählen") }}</option>
                <option v-for="model in aiStore.models" :key="model.id" :value="model.name">
                  {{ modelOptionLabel(model) }}
                </option>
              </select>
            </label>
            <div v-else class="ai-readonly-model">
              <span>{{ ui("Modell") }}</span>
              <strong>{{ aiConfiguredModelLabel }}</strong>
            </div>
            <small>http://localhost:11434/v1</small>
            <strong class="ai-provider-state" :class="aiStore.providerStatus?.state || 'unknown'">
              {{ aiProviderStateLabel }}
            </strong>
            <p v-if="aiStore.isLoadingModels" class="muted-text">{{ ui("Modelle werden geladen.") }}</p>
            <p v-else-if="aiModelListEmpty" class="muted-text">
              {{ ui("Keine Ollama-Modelle installiert. Installiere oder importiere Modelle außerhalb\n              des Autorenprogramms.") }}
            </p>
            <p v-else-if="aiModelMissing" class="error-text">
              {{ ui("Das konfigurierte Modell ist nicht installiert. Bitte ein vorhandenes Modell\n              auswählen.") }}
            </p>
            <button
              v-if="aiStore.providerStatus?.canStart"
              type="button"
              class="primary-action"
              :disabled="aiStore.isStartingProvider"
              @click="startLocalAiProvider"
            >
              {{ t(aiStore.isStartingProvider ? "dynamic.startingOllama" : "dynamic.startOllama") }}
            </button>
          </div>
          <p v-if="aiStore.error" class="error-text">{{ formatUiError(aiStore.error) }}</p>
        </section>

        <section class="settings-info" :aria-label="ui('Projektinformationen')">
          <h3>{{ ui("Projektdatei") }}</h3>
          <dl>
            <div>
              <dt>{{ ui("Speicherort") }}</dt>
              <dd>{{ projectStore.activeProject?.path }}</dd>
            </div>
            <div>
              <dt>{{ ui("Speicherart") }}</dt>
              <dd>{{ projectStore.activeProject?.storage_kind || "file" }}</dd>
            </div>
            <div>
              <dt>{{ ui("Erstellt") }}</dt>
              <dd>{{ formatProjectTimestamp(projectStore.activeProject?.created_at) }}</dd>
            </div>
            <div>
              <dt>{{ ui("Aktualisiert") }}</dt>
              <dd>{{ formatProjectTimestamp(projectStore.activeProject?.updated_at) }}</dd>
            </div>
          </dl>
        </section>
      </section>
</template>
