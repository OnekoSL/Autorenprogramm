<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "StructureView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["structure"];
    if (guard) navigation.register("structure", guard);
    onUnmounted(() => navigation.unregister("structure", guard));
    return context;
  },
});
</script>

<template>
<section
        class="editor structure-editor"
        :aria-label="ui('Story-Struktur')"
      >
        <div v-if="storyStructureStore.displayedStructure" class="structure-workspace-heading">
          <div>
            <p class="eyebrow">{{ t(structureIsPreview ? "dynamic.templatePreview" : "dynamic.activeStructure") }}</p>
            <h2>{{ storyStructureStore.displayedStructure.name }}</h2>
            <p>{{ storyStructureStore.displayedStructure.description }}</p>
            <p class="structure-scaffold-summary">{{ t("structure.scaffoldPreview", {
              areas: t("counts.areas", storyStructureStore.displayedActs.length),
              chapters: t("counts.chapters", structureBeatCount),
            }) }}</p>
          </div>
          <span class="structure-category-badge">
            {{ structureCategoryLabels[storyStructureStore.displayedStructure.category] }}
          </span>
        </div>

        <p v-if="storyStructureStore.isLoading" class="muted-text">{{ ui("Strukturen werden geladen.") }}</p>
        <section v-else-if="!storyStructureStore.displayedStructure" class="empty-panel">
          <h2>{{ ui("Keine Strukturvorlage verfügbar") }}</h2>
          <p>{{ storyStructureStore.error ? formatUiError(storyStructureStore.error) : t("dynamic.templatesFailed") }}</p>
        </section>
        <section
          v-if="storyStructureStore.selectedTemplate?.planningMethods.length"
          class="planning-method-info"
          :aria-label="ui('Informative Arbeitsmethoden')"
        >
          <div class="planning-method-info-heading">
            <div>
              <p class="eyebrow">{{ ui("Arbeitsmethoden") }}</p>
              <h3>{{ ui("Hilfen für die freie Ausarbeitung") }}</h3>
            </div>
            <span>{{ ui("Information, keine Strukturvorlage") }}</span>
          </div>
          <article
            v-for="method in storyStructureStore.selectedTemplate.planningMethods"
            :key="method.id"
            class="planning-method-card"
          >
            <h4>{{ method.name }}</h4>
            <p>{{ method.description }}</p>
            <ol>
              <li v-for="step in method.steps" :key="step">{{ step }}</li>
            </ol>
          </article>
        </section>
        <div v-if="storyStructureStore.displayedStructure" class="structure-act-grid">
          <article v-for="act in storyStructureStore.displayedActs" :key="act.id" class="structure-act-card">
            <header>
              <p class="eyebrow">{{ t("structure.actPhase", { number: act.order + 1 }) }}</p>
              <h3>{{ act.title }}</h3>
              <p>{{ act.description }}</p>
              <small v-if="structureAreaTitle(act.manuscriptAreaId)" class="structure-manuscript-link">
                {{ t("structure.linkedArea", { title: structureAreaTitle(act.manuscriptAreaId) }) }}
              </small>
            </header>
            <div class="structure-beat-list">
              <button
                v-for="beat in act.beats"
                :key="beat.id"
                type="button"
                class="structure-beat-card"
                :class="{ selected: storyStructureStore.selectedBeatId === beat.id }"
                @click="selectStructureBeat(beat.id)"
              >
                <span>
                  <strong>{{ beat.title }}</strong>
                  <small>{{ t(beat.required ? "dynamic.requiredBeat" : "dynamic.optional") }}</small>
                </span>
                <small>{{ t("counts.scenes", beat.sceneIds.length) }} · {{ t("counts.characters", beat.characterIds.length) }} · {{ t(`structure.${beat.status === "in_progress" ? "inProgress" : beat.status || "empty"}`) }}</small>
                <small v-if="structureChapterTitle(beat.manuscriptChapterId)" class="structure-manuscript-link">
                  {{ t("structure.linkedChapter", { title: structureChapterTitle(beat.manuscriptChapterId) }) }}
                </small>
                <small v-if="beat.suggestedPercentageStart != null || beat.suggestedPercentageEnd != null">
                  {{ structurePositionLabel(beat.suggestedPercentageStart, beat.suggestedPercentageEnd) }}
                </small>
              </button>
            </div>
          </article>
        </div>
      </section>
</template>
