<script setup lang="ts">
import { computed, ref } from "vue";
import { chooseProjectOpenPath, chooseProjectSavePath } from "../api/project";
import { isTauriRuntime } from "../api/tauri";
import { useProjectStore } from "../stores/project";

const projectStore = useProjectStore();
const projectName = ref("");
const description = ref("");
const defaultLanguage = ref("de");
const projectPath = ref("");
const localError = ref("");
const hasDesktopBackend = computed(() => isTauriRuntime());
const desktopBackendMessage =
  "Desktop-Backend nicht verbunden. Bitte starte die App mit `npm.cmd run tauri dev`; im normalen Browser koennen keine lokalen Projektdateien angelegt oder geoeffnet werden.";

const canCreateProject = computed(
  () => hasDesktopBackend.value && projectName.value.trim().length > 0 && !projectStore.isLoading,
);

function projectFileName() {
  const normalized = projectName.value.trim().replace(/[<>:"/\\|?*]+/g, "-");
  return normalized || "Neues Projekt";
}

async function chooseSavePath() {
  localError.value = "";

  if (!hasDesktopBackend.value) {
    localError.value = desktopBackendMessage;
    return;
  }

  const selectedPath = await chooseProjectSavePath(projectFileName());

  if (selectedPath) {
    projectPath.value = selectedPath;
  }
}

async function createProject() {
  localError.value = "";

  if (!hasDesktopBackend.value) {
    localError.value = desktopBackendMessage;
    return;
  }

  if (!projectName.value.trim()) {
    localError.value = "Bitte gib einen Projektnamen ein.";
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
  localError.value = "";

  if (!hasDesktopBackend.value) {
    localError.value = desktopBackendMessage;
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
    <section class="project-start-panel" aria-label="Projektstart">
      <header>
        <p class="eyebrow">Autorenprogramm</p>
        <h1>Projektstart</h1>
      </header>

      <section class="start-grid">
        <form class="new-project" @submit.prevent="createProject">
          <div class="section-heading">
            <h2>Neues Projekt</h2>
            <span>Lokal</span>
          </div>

          <label>
            <span>Projektname</span>
            <input v-model="projectName" autocomplete="off" placeholder="Die Chroniken von Eldara" />
          </label>

          <label>
            <span>Beschreibung</span>
            <textarea v-model="description" placeholder="Kurze Orientierung fuer das Projekt." />
          </label>

          <label>
            <span>Standardsprache</span>
            <select v-model="defaultLanguage">
              <option value="de">Deutsch</option>
              <option value="en">Englisch</option>
            </select>
          </label>

          <label>
            <span>Speicherort</span>
            <input v-model="projectPath" readonly placeholder="Noch kein Speicherort gewaehlt" />
          </label>

          <div class="start-actions">
            <button type="button" :disabled="!hasDesktopBackend" @click="chooseSavePath">
              Speicherort auswaehlen
            </button>
            <button type="submit" class="primary-action" :disabled="!canCreateProject">
              Projekt anlegen
            </button>
          </div>
        </form>

        <section class="open-project" aria-label="Projekt oeffnen">
          <div class="section-heading">
            <h2>Zuletzt geoeffnet</h2>
            <span>{{ projectStore.recentProjects.length }}</span>
          </div>

          <div v-if="projectStore.recentProjects.length" class="recent-list">
            <article v-for="project in projectStore.recentProjects" :key="project.path">
              <div>
                <h3>{{ project.name }}</h3>
                <p>{{ project.path }}</p>
              </div>
              <div class="recent-actions">
                <button type="button" @click="projectStore.openProject(project.path)">Oeffnen</button>
                <button type="button" @click="projectStore.removeRecentProject(project.path)">
                  Entfernen
                </button>
              </div>
            </article>
          </div>

          <p v-else class="empty-state">Noch keine zuletzt geoeffneten Projekte.</p>

          <div class="local-file">
            <h2>Lokale Projektdatei</h2>
            <button type="button" :disabled="!hasDesktopBackend" @click="chooseOpenPath">
              Projektdatei oeffnen
            </button>
          </div>
        </section>
      </section>

      <p v-if="!hasDesktopBackend || localError || projectStore.error" class="error-message">
        {{ localError || projectStore.error || desktopBackendMessage }}
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
