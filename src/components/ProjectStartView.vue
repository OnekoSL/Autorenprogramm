<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { setLocale, type ProjectLanguage } from "../i18n";
import { formatUiError, uiError, type UiError } from "../errors";
import { chooseProjectOpenPath, chooseProjectSavePath } from "../api/project";
import { isTauriRuntime } from "../api/tauri";
import { useProjectStore } from "../stores/project";

const projectStore = useProjectStore();
const { t } = useI18n();
const projectName = ref("");
const description = ref("");
const defaultLanguage = ref<ProjectLanguage>("de");
watch(defaultLanguage, (language) => setLocale(language), { immediate: true });
const projectPath = ref("");
const localError = ref<UiError | null>(null);
const hasDesktopBackend = computed(() => isTauriRuntime());
const startError = computed(
  () => localError.value ?? projectStore.error ?? (!hasDesktopBackend.value ? uiError("tauri_unavailable") : null),
);
const startErrorText = computed(() => formatUiError(startError.value));

const canCreateProject = computed(
  () => hasDesktopBackend.value && projectName.value.trim().length > 0 && !projectStore.isLoading,
);

function projectFileName() {
  const normalized = projectName.value.trim().replace(/[<>:"/\\|?*]+/g, "-");
  const baseName = normalized || t("start.defaultName");
  return `${baseName} - ${t("files.project")}`;
}

async function chooseSavePath() {
  localError.value = null;

  if (!hasDesktopBackend.value) {
    localError.value = uiError("tauri_unavailable");
    return;
  }

  const selectedPath = await chooseProjectSavePath(projectFileName());

  if (selectedPath) {
    projectPath.value = selectedPath;
  }
}

async function createProject() {
  localError.value = null;

  if (!hasDesktopBackend.value) {
    localError.value = uiError("tauri_unavailable");
    return;
  }

  if (!projectName.value.trim()) {
    localError.value = uiError("invalid_project_name");
    return;
  }

  let path = projectPath.value;
  if (!path) {
    path = (await chooseProjectSavePath(projectFileName())) ?? "";
  }

  if (!path) {
    return;
  }

  await projectStore.createProject({
    name: projectName.value,
    description: description.value,
    defaultLanguage: defaultLanguage.value,
    path,
  });
}

async function chooseOpenPath() {
  localError.value = null;

  if (!hasDesktopBackend.value) {
    localError.value = uiError("tauri_unavailable");
    return;
  }

  const selectedPath = await chooseProjectOpenPath();

  if (selectedPath) {
    await projectStore.openProject(selectedPath);
  }
}
</script>

<template>
  <main class="project-start">
    <section class="project-start-panel" :aria-label="t('project.start')">
      <header>
        <p class="eyebrow">Autorenprogramm</p>
        <h1>{{ t("project.start") }}</h1>
      </header>

      <section class="start-grid">
        <form class="new-project" @submit.prevent="createProject">
          <div class="section-heading">
            <h2>{{ t("project.new") }}</h2>
            <span>{{ t("project.local") }}</span>
          </div>

          <label>
            <span>{{ t("project.name") }}</span>
            <input v-model="projectName" autocomplete="off" :placeholder="t('start.namePlaceholder')" />
          </label>

          <label>
            <span>{{ t("project.description") }}</span>
            <textarea v-model="description" :placeholder="t('start.descriptionPlaceholder')" />
          </label>

          <label>
            <span>{{ t("project.language") }}</span>
            <select v-model="defaultLanguage">
              <option value="de">{{ t("project.german") }}</option>
              <option value="en">{{ t("project.english") }}</option>
            </select>
          </label>

          <div class="project-default-info">
            <span>{{ t("project.defaultStructure") }}</span>
            <strong>{{ projectStore.projectDefaults?.storyStructureTemplateName || t("start.freeStructure") }}</strong>
            <small>{{ t("project.appliedAutomatically") }}</small>
          </div>

          <label>
            <span>{{ t("project.location") }}</span>
            <input v-model="projectPath" readonly :placeholder="t('project.noLocation')" />
          </label>

          <div class="start-actions">
            <button type="button" :disabled="!hasDesktopBackend" @click="chooseSavePath">
              {{ t("project.choose") }}
            </button>
            <button type="submit" class="primary-action" :disabled="!canCreateProject">
              {{ t("project.create") }}
            </button>
          </div>
        </form>

        <section class="open-project" :aria-label="t('project.open')">
          <div class="section-heading">
            <h2>{{ t("project.recent") }}</h2>
            <span>{{ projectStore.recentProjects.length }}</span>
          </div>

          <div v-if="projectStore.recentProjects.length" class="recent-list">
            <article v-for="project in projectStore.recentProjects" :key="project.path">
              <div>
                <h3>{{ project.name }}</h3>
                <p>{{ project.path }}</p>
              </div>
              <div class="recent-actions">
                <button type="button" @click="projectStore.openProject(project.path)">{{ t("common.open") }}</button>
                <button type="button" @click="projectStore.removeRecentProject(project.path)">
                  {{ t("project.remove") }}
                </button>
              </div>
            </article>
          </div>

          <p v-else class="empty-state">{{ t("project.noRecent") }}</p>

          <div class="local-file">
            <h2>{{ t("project.localFile") }}</h2>
            <button type="button" :disabled="!hasDesktopBackend" @click="chooseOpenPath">
              {{ t("project.open") }}
            </button>
          </div>
        </section>
      </section>

      <p v-if="startError" class="error-message">
        {{ startErrorText }}
      </p>
    </section>
  </main>
</template>

<style scoped>
.project-start {
  display: grid;
  min-height: 100vh;
  place-items: center;
  padding: 36px;
  background: #f2f0eb;
}

.project-start-panel {
  width: min(1040px, 100%);
  border: 1px solid #d7d2c7;
  border-radius: 8px;
  background: #fbfaf7;
  padding: 26px;
}

.project-start-panel > header {
  margin-bottom: 24px;
}

.project-start-panel h1,
.section-heading h2,
.local-file h2,
.recent-list h3 {
  margin: 0;
  letter-spacing: 0;
}

.project-start-panel h1 {
  font-size: 26px;
}

.start-grid {
  display: grid;
  grid-template-columns: minmax(360px, 0.9fr) minmax(420px, 1.1fr);
  gap: 24px;
}

.new-project,
.open-project {
  min-width: 0;
}

.new-project {
  display: grid;
  gap: 14px;
}

.section-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

.section-heading h2,
.local-file h2 {
  font-size: 18px;
}

.section-heading span {
  color: #6c746e;
  font-size: 12px;
  font-weight: 650;
  text-transform: uppercase;
}

.new-project textarea {
  height: 96px;
}

.project-default-info {
  display: grid;
  gap: 3px;
  border: 1px solid #d7d2c7;
  border-radius: 6px;
  background: #f3f6f3;
  padding: 10px 12px;
}

.project-default-info span,
.project-default-info small {
  color: #66736c;
  font-size: 12px;
}

.start-actions,
.recent-actions {
  display: flex;
  gap: 8px;
}

.start-actions {
  justify-content: flex-end;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.recent-list {
  display: grid;
  gap: 10px;
  margin-top: 14px;
}

.recent-list article {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 14px;
  align-items: center;
  border: 1px solid #d7d2c7;
  border-radius: 8px;
  background: #fffdfa;
  padding: 12px;
}

.recent-list h3 {
  font-size: 15px;
}

.recent-list p,
.empty-state {
  margin: 4px 0 0;
  color: #6c746e;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-actions {
  justify-content: flex-end;
}

.local-file {
  margin-top: 24px;
  border-top: 1px solid #d7d2c7;
  padding-top: 18px;
}

.local-file button {
  margin-top: 12px;
}

.error-message {
  margin: 18px 0 0;
  border: 1px solid #d9a59c;
  border-radius: 6px;
  background: #fff0ed;
  color: #8a2f22;
  padding: 10px 12px;
}
</style>
