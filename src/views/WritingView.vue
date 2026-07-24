<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "WritingView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["writing"];
    if (guard) navigation.register("writing", guard);
    onUnmounted(() => navigation.unregister("writing", guard));
    return context;
  },
});
</script>

<template>
<section class="editor" :aria-label="ui('Aktuelle Szene')">
        <template v-if="sceneDraft.id">
          <div class="scene-meta">
            <label>
              <span>{{ ui("Titel") }}</span>
              <input v-model="sceneDraft.title" />
            </label>

            <label>
              <span>{{ ui("Status") }}</span>
              <select v-model="sceneDraft.status">
                <option v-for="status in sceneStatuses" :key="status.value" :value="status.value">
                  {{ status.label }}
                </option>
              </select>
            </label>

            <div class="scene-info-chips" :aria-label="ui('Szeneninformationen')">
              <span>{{ t("counts.words", manuscriptStore.wordCount) }}</span>
              <span>{{ manuscriptStore.activeLocation }}</span>
              <span>{{ t("counts.cards", writingBibleSceneLinks.length) }}</span>
            </div>
          </div>

          <textarea
            ref="sceneTextAreaRef"
            v-model="sceneDraft.text"
            :style="sceneEditorStyle"
            spellcheck="true"
            :aria-label="ui('Markdown-Editor')"
            @select="updateSceneTextSelection"
            @keydown="handleSceneTextKeydown"
            @keyup="updateSceneTextSelection"
            @mouseup="updateSceneTextSelection"
          />
        </template>

        <section v-else class="empty-editor">
          <h2>{{ ui("Keine Szene ausgewählt") }}</h2>
          <p>{{ ui("Lege eine Szene an, um mit dem Manuskript zu beginnen.") }}</p>
          <button type="button" class="primary-action" @click="createSceneForActiveChapter">
            {{ ui("Szene anlegen") }}
          </button>
        </section>
      </section>
</template>
