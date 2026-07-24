<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "ProjectBibleView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["bible"];
    if (guard) navigation.register("bible", guard);
    onUnmounted(() => navigation.unregister("bible", guard));
    return context;
  },
});
</script>

<template>
<section class="editor bible-editor" :aria-label="ui('Ausgewählte Projektbibel-Karte')">
        <template v-if="selectedCard">
          <div class="card-meta">
            <div>
              <p class="eyebrow">{{ selectedCardTypeLabel }}</p>
              <h2>{{ selectedCard.title || cardTypeSingular(selectedCard.targetType) }}</h2>
            </div>
            <button type="button" class="danger-action" @click="archiveSelectedBibleCard">
              {{ ui("Archivieren") }}
            </button>
          </div>

          <div v-if="cardDraft.targetType === 'faction'" class="bible-form card-editor-form">
            <label><span>{{ ui("Name") }}</span><input v-model="cardDraft.name" /></label>
            <label><span>{{ ui("Art") }}</span><input v-model="cardDraft.role" :placeholder="ui('Orden, Regierung, Konzern …')" /></label>
            <label><span>{{ ui("Beschreibung") }}</span><textarea v-model="cardDraft.description" /></label>
            <label><span>{{ ui("Ziele") }}</span><textarea v-model="cardDraft.body" /></label>
            <label><span>{{ ui("Notizen") }}</span><textarea v-model="cardDraft.notes" /></label>
          </div>

          <div v-else-if="cardDraft.targetType === 'event'" class="bible-form card-editor-form">
            <label><span>{{ ui("Titel") }}</span><input v-model="cardDraft.title" /></label>
            <label><span>{{ ui("Zeitangabe") }}</span><input v-model="cardDraft.role" :placeholder="ui('Dritter Winter, Jahr 2147 …')" /></label>
            <label><span>{{ ui("Zusammenfassung") }}</span><textarea v-model="cardDraft.description" /></label>
            <label><span>{{ ui("Verlauf / Ausgang") }}</span><textarea v-model="cardDraft.body" /></label>
            <label><span>{{ ui("Notizen") }}</span><textarea v-model="cardDraft.notes" /></label>
            <div class="event-order-actions"><button type="button" @click="moveSelectedEvent(-1)">{{ ui("Früher") }}</button><button type="button" @click="moveSelectedEvent(1)">{{ ui("Später") }}</button></div>
          </div>

          <div v-else-if="cardDraft.targetType !== 'note'" class="bible-form card-editor-form">
            <label>
              <span>{{ ui("Name") }}</span>
              <input v-model="cardDraft.name" />
            </label>

            <label v-if="cardDraft.targetType === 'character'">
              <span>{{ ui("Rolle") }}</span>
              <input v-model="cardDraft.role" />
            </label>

            <label>
              <span>{{ ui("Beschreibung") }}</span>
              <textarea v-model="cardDraft.description" />
            </label>

            <label>
              <span>{{ ui("Notizen") }}</span>
              <textarea v-model="cardDraft.notes" />
            </label>
          </div>

          <div v-else class="bible-form card-editor-form">
            <label>
              <span>{{ ui("Titel") }}</span>
              <input v-model="cardDraft.title" />
            </label>

            <label>
              <span>{{ ui("Notiztyp") }}</span>
              <select v-model="cardDraft.noteType">
                <option v-for="noteType in noteTypes" :key="noteType.id" :value="noteType.id">
                  {{ noteType.label }}
                </option>
              </select>
            </label>

            <label>
              <span>{{ ui("Inhalt") }}</span>
              <textarea v-model="cardDraft.body" />
            </label>
          </div>
        </template>

        <section v-else class="empty-editor">
          <h2>{{ ui("Keine Karte ausgewählt") }}</h2>
          <p>{{ ui("Lege eine Karte an, um Projektwissen zu erfassen.") }}</p>
          <button type="button" class="primary-action" @click="createBibleCard">
            {{ ui("Karte anlegen") }}
          </button>
        </section>
      </section>
</template>
