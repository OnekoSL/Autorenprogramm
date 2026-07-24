<script lang="ts">
import { defineComponent } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";

export default defineComponent({
  name: "WorkspaceSidebarPanel",
  setup() {
    return useWorkspaceViewContext();
  },
});
</script>

<template>
<button
        type="button"
        class="resize-handle"
        :aria-label="ui('Seitenleiste verbreitern oder verkleinern')"
        @pointerdown="startPanelResize('sidebar', $event)"
      />

<aside class="sidebar" :aria-label="ui('Rechte Seitenleiste')">
        <div
          v-if="workspace.activeSection !== 'worldbuilding'"
          class="tabs"
          role="tablist"
          :aria-label="ui('Seitenleistenmodus')"
        >
          <button
            v-for="tab in sidebarTabs"
            :key="tab.id"
            type="button"
            :class="{ active: workspace.sidebarTab === tab.id }"
            @click="workspace.setSidebarTab(tab.id)"
          >
            {{ tab.label }}
          </button>
        </div>

        <div v-else-if="workspace.activeSection === 'worldbuilding'" class="tabs"><button :class="{ active: workspace.worldSidebarTab === 'guidance' }" @click="workspace.setWorldSidebarTab('guidance')">{{ ui("Leitfragen") }}</button><button :class="{ active: workspace.worldSidebarTab === 'scenes' }" @click="workspace.setWorldSidebarTab('scenes')">{{ ui("Szenen") }}</button><button :class="{ active: workspace.worldSidebarTab === 'relationships' }" @click="workspace.setWorldSidebarTab('relationships')">{{ ui("Beziehungen") }}</button></div>

        <div class="panel-heading">
          <h2>{{ workspace.activeSection === 'worldbuilding' ? ui("Weltsystem") : sidebarTitle }}</h2>
          <span
            v-if="workspace.sidebarTab === 'ai'"
            class="panel-heading-status ai-provider-state"
            :class="aiStore.providerStatus?.state || 'unknown'"
          >
            {{ aiProviderStateLabel }}
          </span>
          <span v-else-if="workspace.activeSection === 'structure'">{{ ui("Beat") }}</span>
          <span v-else>{{ workspace.sidebarTab }}</span>
        </div>

        <template v-if="workspace.activeSection === 'worldbuilding'">
          <section v-if="workspace.worldSidebarTab === 'guidance'" class="context-list">
            <template v-if="worldbuildingStore.selectedSystem">
              <h3>{{ selectedWorldTemplate?.name || t("dynamic.generalGuidance") }}</h3>
              <ul><li v-for="question in selectedWorldTemplate?.guidingQuestions || []" :key="question">{{ question }}</li></ul>
              <p class="muted-text">{{ ui("Leitfragen sind optional.") }}</p>
              <h3>{{ ui("Noch offen") }}</h3>
              <p v-if="worldSystemNameMissing" class="error-text">{{ ui("Name · erforderlich") }}</p>
              <p v-if="openWorldSystemFields.length" class="muted-text">{{ locale === "en" ? `${openWorldSystemFields.length} optional field(s) are empty:` : `${openWorldSystemFields.length} optionale Felder sind leer:` }}</p>
              <ul v-if="openWorldSystemFields.length"><li v-for="field in openWorldSystemFields" :key="field.key">{{ field.label }}</li></ul>
              <p v-if="!worldSystemNameMissing && !openWorldSystemFields.length" class="muted-text">{{ ui("Alle angezeigten Felder sind ausgefüllt.") }}</p>
              <p class="muted-text">{{ ui("Leere optionale Felder müssen nicht ausgefüllt werden.") }}</p>
            </template>
            <p v-else class="muted-text">{{ ui("Wähle oder erstelle ein Weltsystem, um Leitfragen und offene Felder zu sehen.") }}</p>
          </section>
          <section v-else-if="workspace.worldSidebarTab === 'scenes'" class="scene-assignment-list"><label v-for="scene in manuscriptStore.scenes" :key="scene.id" class="scene-assignment-row"><input type="checkbox" :checked="projectBibleStore.snapshot?.sceneLinks.some((link) => link.sceneId === scene.id && link.targetType === 'world_system' && link.targetId === worldbuildingStore.selectedSystemId)" @change="toggleSceneWorldSystem(worldbuildingStore.selectedSystemId, ($event.target as HTMLInputElement).checked, scene.id)" /><span><strong>{{ scene.title }}</strong><small>{{ scene.summary || ui("Keine Kurzbeschreibung") }}</small></span></label></section>
          <section v-else class="context-list"><h3>{{ ui("Beziehungen") }}</h3><div v-for="relation in projectBibleStore.snapshot?.relationships.filter((item) => (item.sourceType === 'world_system' && item.sourceId === worldbuildingStore.selectedSystemId) || (item.targetType === 'world_system' && item.targetId === worldbuildingStore.selectedSystemId)) || []" :key="relation.id" class="relationship-row"><button type="button" class="relationship-main" @click="editWorldRelationship(relation)"><strong>{{ relationshipLabel(relation, 'world_system', worldbuildingStore.selectedSystemId) }}</strong><span>{{ relationshipCard(relation, 'world_system', worldbuildingStore.selectedSystemId).card?.title || t("dynamic.archivedCard") }}</span><small v-if="relation.description">{{ relation.description }}</small></button><button class="danger-action" @click="projectBibleStore.removeRelationship(relation.id)">×</button></div><div class="relationship-form"><select v-model="relationshipTargetType" :disabled="Boolean(relationshipEditId)"><option v-for="type in projectBibleCardTypes" :key="type.id" :value="type.id">{{ type.label }}</option><option value="world_system">{{ ui("Weltsysteme") }}</option></select><input v-model="relationshipSearch" :placeholder="ui('Ziel suchen')" :disabled="Boolean(relationshipEditId)" /><select v-model="relationshipTargetId" :disabled="Boolean(relationshipEditId)"><option value="">{{ ui("Ziel wählen") }}</option><option v-for="card in relationshipCandidates" :key="card.id" :value="card.id">{{ card.title }}</option></select><select v-model="relationshipType"><option v-for="type in availableRelationshipTypes" :key="type.id" :value="type.id">{{ type.label }}</option></select><textarea v-model="relationshipDescription" :placeholder="ui('Optionale Beschreibung')" /><button type="button" class="primary-action" :disabled="!relationshipTargetId" @click="addWorldRelationship">{{ relationshipEditId ? t("dynamic.saveRelationship") : t("labels.addRelationship") }}</button></div></section>
        </template>

        <template v-else-if="workspace.activeSection === 'structure' && workspace.sidebarTab !== 'world'">
          <section v-if="storyStructureStore.selectedBeat" class="structure-detail-panel">
            <p class="eyebrow">{{ storyStructureStore.selectedAct?.title }}</p>
            <template v-if="!structureIsPreview">
              <label><span>{{ ui("Titel") }}</span><input v-model="beatDraft.title" /></label>
              <label><span>{{ ui("Beschreibung") }}</span><textarea v-model="beatDraft.description" /></label>
              <label><span>{{ ui("Eigene Zusammenfassung") }}</span><textarea v-model="beatDraft.summary" /></label>
              <label><span>{{ ui("Notizen") }}</span><textarea v-model="beatDraft.notes" /></label>
              <label><span>{{ ui("Status") }}</span><select v-model="beatDraft.status"><option value="empty">{{ ui("Noch leer") }}</option><option value="planned">{{ ui("Geplant") }}</option><option value="in_progress">{{ ui("In Arbeit") }}</option><option value="complete">{{ ui("Abgeschlossen") }}</option></select></label>
              <h3>{{ ui("Szenen") }}</h3>
              <div class="scene-assignment-list"><label v-for="scene in manuscriptStore.scenes" :key="scene.id" class="scene-assignment-row"><input type="checkbox" :checked="storyStructureStore.selectedBeat.sceneIds.includes(scene.id)" @change="toggleBeatScene(scene.id, ($event.target as HTMLInputElement).checked)" /><span><strong>{{ scene.title }}</strong><small>{{ scene.summary || ui("Keine Kurzbeschreibung") }}</small></span></label></div>
              <h3>{{ ui("Figuren") }}</h3>
              <div class="scene-assignment-list"><label v-for="character in structureCharacters" :key="character.id" class="scene-assignment-row"><input type="checkbox" :checked="storyStructureStore.selectedBeat.characterIds.includes(character.id)" @change="toggleBeatCharacter(character.id, ($event.target as HTMLInputElement).checked)" /><span><strong>{{ character.name }}</strong><small>{{ character.role || t("labels.noRole") }}</small></span></label></div>
            </template>
            <p v-else class="muted-text">{{ ui("Wende die Vorlage an, um ihre Beats zu bearbeiten und zu verknüpfen.") }}</p>

            <h3>{{ ui("Leitfragen") }}</h3>
            <ul class="structure-question-list">
              <li v-for="question in storyStructureStore.selectedBeat.guidingQuestions" :key="question">
                {{ question }}
              </li>
            </ul>

            <h3>{{ ui("Position") }}</h3>
            <p>
              {{ structurePositionLabel(
                storyStructureStore.selectedBeat.suggestedPercentageStart,
                storyStructureStore.selectedBeat.suggestedPercentageEnd,
              ) }}
            </p>
            <p class="muted-text">{{ ui("Positionen sind Orientierung, keine feste Regel.") }}</p>

          </section>
          <p v-else class="muted-text">{{ ui("Wähle einen Beat aus, um Beschreibung und Leitfragen zu sehen.") }}</p>
        </template>

        <template v-else-if="workspace.activeSection === 'bible'">
          <section v-if="workspace.sidebarTab === 'context'" class="scene-assignment-list">
            <p v-if="!selectedCard" class="muted-text">{{ ui("Keine Karte ausgewählt") }}</p>
            <template v-else>
              <label
                v-for="scene in manuscriptStore.scenes"
                :key="scene.id"
                class="scene-assignment-row"
              >
                <input
                  type="checkbox"
                  :checked="isSceneLinkedToSelectedCard(scene.id)"
                  @change="toggleSelectedCardScene(scene.id, $event)"
                />
                <span>
                  <strong>{{ scene.title }}</strong>
                  <small>{{ scene.summary || ui("Keine Kurzbeschreibung") }}</small>
                </span>
              </label>
              <p v-if="!manuscriptStore.scenes.length" class="muted-text">
                {{ ui("Noch keine Szenen vorhanden.") }}
              </p>
            </template>
          </section>

          <section v-else-if="workspace.sidebarTab === 'bible'" class="context-list">
            <template v-if="selectedCard">
              <h3>{{ ui("Typ") }}</h3>
              <p>{{ cardTypeSingular(selectedCard.targetType) }}</p>
              <h3>{{ ui("Titel") }}</h3>
              <p>{{ selectedCard.title }}</p>
              <h3>{{ ui("Zusammenfassung") }}</h3>
              <p>{{ selectedCard.subtitle || t("dynamic.noSummary") }}</p>
              <h3>{{ ui("Szenen") }}</h3>
              <p>{{ t("counts.linked", projectBibleStore.selectedCardLinks.length) }}</p>
              <h3>{{ ui("Beziehungen") }}</h3>
              <div v-for="item in projectBibleStore.selectedCardRelationships" :key="item.id" class="relationship-row">
                <button type="button" class="relationship-main" @click="editBibleRelationship(item)">
                  <strong>{{ relationshipLabel(item) }}</strong>
                  <span>{{ relationshipCard(item).card?.title || t("dynamic.archivedCard") }}</span>
                  <small v-if="item.description">{{ item.description }}</small>
                </button>
                <button type="button" class="danger-action" @click="projectBibleStore.removeRelationship(item.id)">×</button>
              </div>
              <p v-if="!projectBibleStore.selectedCardRelationships.length" class="muted-text">{{ ui("Noch keine Beziehungen.") }}</p>
              <div class="relationship-form">
                <select v-model="relationshipTargetType" :disabled="Boolean(relationshipEditId)"><option v-for="type in projectBibleCardTypes" :key="type.id" :value="type.id">{{ type.label }}</option><option value="world_system">{{ ui("Weltsysteme") }}</option></select>
                <input v-model="relationshipSearch" :placeholder="ui('Zielkarte suchen')" :disabled="Boolean(relationshipEditId)" />
                <select v-model="relationshipTargetId" :disabled="Boolean(relationshipEditId)"><option value="">{{ ui("Zielkarte wählen") }}</option><option v-for="card in relationshipCandidates" :key="card.id" :value="card.id">{{ card.title }}</option></select>
                <select v-model="relationshipType"><option v-for="type in availableRelationshipTypes" :key="type.id" :value="type.id">{{ type.label }}</option></select>
                <textarea v-model="relationshipDescription" :placeholder="ui('Optionale Beschreibung')" />
                <button type="button" class="primary-action" :disabled="!relationshipTargetId || projectBibleStore.isSaving" @click="addBibleRelationship">{{ relationshipEditId ? t("dynamic.saveRelationship") : t("labels.addRelationship") }}</button>
              </div>
            </template>
            <p v-else class="muted-text">{{ ui("Keine Karte ausgewählt") }}</p>
          </section>

          <section v-else-if="workspace.sidebarTab === 'world'" class="context-list">
            <h3>{{ ui("Verbundene Weltsysteme") }}</h3><article v-for="system in selectedCardWorldSystems" :key="system.id" class="world-summary-card"><strong>{{ system.name }}</strong><small>{{ worldCategoryLabel(system.category) }}</small><p>{{ system.principle || system.rules || t("labels.noBriefInfo") }}</p><button type="button" @click="openWorldSystem(system.id)">{{ ui("Im Worldbuilding öffnen") }}</button></article><p v-if="!selectedCardWorldSystems.length" class="muted-text">{{ ui("Keine Weltsysteme verbunden.") }}</p>
            <select @change="linkSelectedCardToWorld(($event.target as HTMLSelectElement).value); ($event.target as HTMLSelectElement).value = ''"><option value="">{{ ui("Weltsystem verknüpfen") }}</option><option v-for="system in worldbuildingStore.systems.filter((item) => !selectedCardWorldSystems.some((linked) => linked.id === item.id))" :key="system.id" :value="system.id">{{ system.name }}</option></select>
          </section>

          <section v-else class="ai-panel">
            <label>
              <span>{{ ui("Aktion") }}</span>
              <select v-model="bibleAiMode">
                <option value="generate">{{ ui("Neue Karte aus Stichworten") }}</option>
                <option value="edit">{{ ui("Bestehende Karte bearbeiten/erweitern") }}</option>
              </select>
            </label>

            <label>
              <span>{{ t(bibleAiMode === "edit" ? "dynamic.instruction" : "dynamic.keywords") }}</span>
              <textarea
                v-model="bibleAiKeywords"
                :placeholder="t(bibleAiMode === 'edit' ? 'labels.editingPlaceholder' : 'labels.keywordsPlaceholder')"
              />
            </label>

            <div class="context-list compact">
              <p>{{ selectedCard?.title || t("dynamic.noCard") }}</p>
              <p>{{ cardTypeSingular(cardDraft.targetType) }}</p>
              <p>{{ t("counts.linkedScenes", projectBibleStore.selectedCardLinks.length) }}</p>
              <p v-if="bibleAiMode === 'edit'">
                {{ ui("Gefüllte Felder dürfen entsprechend dem Auftrag geändert werden.") }}
              </p>
            </div>

            <button
              type="button"
              class="primary-action"
              :disabled="!canRunBibleAi"
              @click="runBibleAi"
            >
              {{ t(aiStore.isRunningBible ? "dynamic.creatingSuggestion" : "dynamic.run") }}
            </button>

            <p v-if="aiStore.error" class="error-text">{{ formatUiError(aiStore.error) }}</p>
            <p v-else-if="!bibleAiKeywords.trim()" class="muted-text">
              {{ ui("Gib Stichworte ein, bevor die Karten-KI ausgeführt wird.") }}
            </p>
            <p v-else-if="!aiStore.isProviderReachable" class="muted-text">
              {{ ui("Die lokale KI muss erreichbar sein, bevor diese Aktion ausgeführt werden kann.") }}
            </p>
            <p v-else-if="aiModelListEmpty" class="muted-text">
              {{ ui("Es ist noch kein lokales Modell für diese Aktion verfügbar.") }}
            </p>
            <p v-else-if="aiModelMissing" class="muted-text">
              {{ ui("Bitte zuerst ein installiertes Modell auswählen.") }}
            </p>
            <p v-else-if="bibleAiMode === 'edit' && !selectedCard" class="muted-text">
              {{ ui("Bitte zuerst eine Projektbibel-Karte auswählen.") }}
            </p>

            <article v-if="aiStore.activeBibleSuggestion && bibleAiHasDraft" class="ai-suggestion">
              <h3>{{ ui("Vorschlag") }}</h3>
              <div v-if="aiStore.activeBibleSuggestion.mode === 'edit'" class="bible-ai-diff-list">
                <p v-if="!bibleAiChangedFields.length" class="muted-text">
                  {{ ui("Die KI hat keine Felder geändert.") }}
                </p>
                <article v-for="field in bibleAiChangedFields" :key="field.key" class="bible-ai-diff-field">
                  <label class="bible-ai-field-choice">
                    <input v-model="bibleAiSelectedFields" type="checkbox" :value="field.key" />
                    <strong>{{ field.label }} · {{ ui("Übernehmen") }}</strong>
                  </label>
                  <div class="bible-ai-original">
                    <small>{{ ui("Original") }}</small>
                    <p>{{ aiStore.activeBibleSuggestion.sourceDraft?.[field.key] || "—" }}</p>
                  </div>
                  <label>
                    <span>{{ ui("KI-Vorschlag") }}</span>
                    <textarea v-if="field.multiline" v-model="bibleAiDraft[field.key]" />
                    <select v-else-if="field.key === 'noteType'" v-model="bibleAiDraft.noteType">
                      <option v-for="noteType in noteTypes" :key="noteType.id" :value="noteType.id">{{ noteType.label }}</option>
                    </select>
                    <input v-else v-model="bibleAiDraft[field.key]" />
                  </label>
                </article>
              </div>
              <div v-else-if="cardDraft.targetType === 'faction'" class="bible-form card-editor-form">
                <label><span>{{ ui("Name") }}</span><input v-model="bibleAiDraft.name" /></label><label><span>{{ ui("Art") }}</span><input v-model="bibleAiDraft.role" /></label><label><span>{{ ui("Beschreibung") }}</span><textarea v-model="bibleAiDraft.description" /></label><label><span>{{ ui("Ziele") }}</span><textarea v-model="bibleAiDraft.body" /></label><label><span>{{ ui("Notizen") }}</span><textarea v-model="bibleAiDraft.notes" /></label>
              </div>
              <div v-else-if="cardDraft.targetType === 'event'" class="bible-form card-editor-form">
                <label><span>{{ ui("Titel") }}</span><input v-model="bibleAiDraft.title" /></label><label><span>{{ ui("Zeitangabe") }}</span><input v-model="bibleAiDraft.role" /></label><label><span>{{ ui("Zusammenfassung") }}</span><textarea v-model="bibleAiDraft.description" /></label><label><span>{{ ui("Verlauf / Ausgang") }}</span><textarea v-model="bibleAiDraft.body" /></label><label><span>{{ ui("Notizen") }}</span><textarea v-model="bibleAiDraft.notes" /></label>
              </div>
              <div v-else-if="cardDraft.targetType !== 'note'" class="bible-form card-editor-form">
                <label>
                  <span>{{ ui("Name") }}</span>
                  <input v-model="bibleAiDraft.name" />
                </label>
                <label v-if="cardDraft.targetType === 'character'">
                  <span>{{ ui("Rolle") }}</span>
                  <input v-model="bibleAiDraft.role" />
                </label>
                <label>
                  <span>{{ ui("Beschreibung") }}</span>
                  <textarea v-model="bibleAiDraft.description" />
                </label>
                <label>
                  <span>{{ ui("Notizen") }}</span>
                  <textarea v-model="bibleAiDraft.notes" />
                </label>
              </div>
              <div v-else class="bible-form card-editor-form">
                <label>
                  <span>{{ ui("Titel") }}</span>
                  <input v-model="bibleAiDraft.title" />
                </label>
                <label>
                  <span>{{ ui("Notiztyp") }}</span>
                  <select v-model="bibleAiDraft.noteType">
                    <option v-for="noteType in noteTypes" :key="noteType.id" :value="noteType.id">
                      {{ noteType.label }}
                    </option>
                  </select>
                </label>
                <label>
                  <span>{{ ui("Inhalt") }}</span>
                  <textarea v-model="bibleAiDraft.body" />
                </label>
              </div>
              <p v-if="bibleAiDuplicateName" class="muted-text">
                {{ ui("Es gibt bereits eine Karte mit diesem Namen.") }}
              </p>
              <small>{{ aiStore.activeBibleSuggestion.status }}</small>
              <div class="ai-suggestion-actions">
                <button
                  type="button"
                  class="primary-action"
                  :disabled="aiStore.activeBibleSuggestion.status !== 'draft' || aiStore.isSaving || (aiStore.activeBibleSuggestion.mode === 'edit' && !bibleAiSelectedFields.length)"
                  @click="acceptBibleAiSuggestion"
                >
                  {{ ui("Übernehmen") }}
                </button>
                <button
                  type="button"
                  class="danger-action"
                  :disabled="aiStore.activeBibleSuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="rejectBibleAiSuggestion"
                >
                  {{ ui("Verwerfen") }}
                </button>
              </div>
            </article>
          </section>
        </template>

        <template v-else>
          <section v-if="workspace.sidebarTab === 'context'" class="context-list">
            <h3>{{ ui("Aktuelle Szene") }}</h3>
            <p>{{ sceneDraft.title || t("dynamic.noScene") }}</p>
            <h3>{{ ui("Status") }}</h3>
            <p>{{ sceneStatusLabel(sceneDraft.status) }}</p>
            <h3>{{ ui("Kurzbeschreibung") }}</h3>
            <p>{{ sceneDraft.summary || ui("Keine Kurzbeschreibung") }}</p>
            <h3>{{ ui("Struktur") }}</h3>
            <div v-if="activeSceneStructureAssignments.length" class="structure-context-summary">
              <p><strong>{{ ui("Romanstruktur:") }}</strong> {{ storyStructureStore.activeStructure?.name }}</p>
              <p v-for="assignment in activeSceneStructureAssignments" :key="assignment.beat.id"><strong>{{ assignment.act.title }}:</strong> {{ assignment.beat.title }}</p>
            </div>
            <p v-else-if="storyStructureStore.activeStructure">
              {{ ui("Keinem Struktur-Beat zugeordnet") }}
            </p>
            <p v-else>{{ ui("Keine aktive Romanstruktur") }}</p>
            <template v-if="storyStructureStore.activeStructure && manuscriptStore.activeSceneId">
              <h3>{{ ui("Beat-Zuordnung") }}</h3>
              <div class="scene-assignment-list"><label v-for="act in storyStructureStore.activeStructure.acts" :key="act.id"><strong>{{ act.title }}</strong><span v-for="beat in act.beats" :key="beat.id" class="scene-assignment-row"><input type="checkbox" :checked="beat.sceneIds.includes(manuscriptStore.activeSceneId)" @change="toggleActiveSceneBeat(beat.id, ($event.target as HTMLInputElement).checked)" /><span>{{ beat.title }}</span></span></label></div>
            </template>
          </section>

          <section v-else-if="workspace.sidebarTab === 'bible'" class="writing-bible-panel">
            <p v-if="!manuscriptStore.activeSceneId" class="muted-text">{{ ui("Keine Szene ausgewählt") }}</p>

            <template v-else>
              <section class="writing-bible-add-panel">
                <div class="writing-bible-add-row">
                  <select v-model="writingBibleAddType" :aria-label="ui('Kartentyp')">
                    <option
                      v-for="cardType in projectBibleCardTypes"
                      :key="cardType.id"
                      :value="cardType.id"
                    >
                      {{ cardType.label }}
                    </option>
                  </select>
                  <button
                    type="button"
                    class="primary-action"
                    :disabled="projectBibleStore.isSaving"
                    @click="createWritingBibleCard"
                  >
                    {{ ui("Neue Karte") }}
                  </button>
                </div>

                <input
                  v-model="writingBibleSearchQuery"
                  class="writing-bible-search"
                  :placeholder="ui('Karte suchen')"
                  :aria-label="ui('Karte suchen')"
                />

                <div v-if="writingBibleAvailableCards.length" class="writing-bible-add-list">
                  <button
                    v-for="card in writingBibleAvailableCards"
                    :key="projectBibleCardKey(card)"
                    type="button"
                    class="writing-bible-add-card"
                    @click="addWritingBibleCard(card)"
                  >
                    <span>
                      <strong>{{ card.title || cardTypeSingular(card.targetType) }}</strong>
                      <small>{{ card.subtitle || cardTypeSingular(card.targetType) }}</small>
                    </span>
                    <small>{{ ui("Hinzufügen") }}</small>
                  </button>
                </div>
                <p v-else class="muted-text">{{ ui("Keine weiteren Karten") }}</p>
              </section>

              <p v-if="!writingBibleGroups.length" class="muted-text">{{ ui("Keine Karten zu dieser Szene") }}</p>

              <section
                v-for="group in writingBibleGroups"
                v-else
                :key="group.type"
                class="writing-bible-group"
              >
                <h3>{{ group.label }}</h3>
                <article
                  v-for="card in group.cards"
                  :key="projectBibleCardKey(card)"
                  class="writing-bible-card"
                  :class="{ open: isWritingBibleCardOpen(card) }"
                >
                  <button type="button" @click="toggleWritingBibleCard(card)">
                    <span>{{ card.title || cardTypeSingular(card.targetType) }}</span>
                    <small>{{ card.subtitle || cardTypeSingular(card.targetType) }}</small>
                  </button>

                  <div v-if="isWritingBibleCardOpen(card)" class="writing-bible-card-detail">
                    <div
                      v-if="cardDraft.id === card.id && cardDraft.targetType === card.targetType"
                      class="bible-form writing-bible-card-form"
                    >
                      <template v-if="cardDraft.targetType !== 'note'">
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
                      </template>

                      <template v-else>
                        <label>
                          <span>{{ ui("Titel") }}</span>
                          <input v-model="cardDraft.title" />
                        </label>
                        <label>
                          <span>{{ ui("Notiztyp") }}</span>
                          <select v-model="cardDraft.noteType">
                            <option
                              v-for="noteType in noteTypes"
                              :key="noteType.id"
                              :value="noteType.id"
                            >
                              {{ noteType.label }}
                            </option>
                          </select>
                        </label>
                        <label>
                          <span>{{ ui("Inhalt") }}</span>
                          <textarea v-model="cardDraft.body" />
                        </label>
                      </template>
                    </div>
                  </div>
                </article>
              </section>
            </template>
          </section>

          <section v-else-if="workspace.sidebarTab === 'world'" class="context-list">
            <h3>{{ t(workspace.activeSection === 'writing' ? 'dynamic.worldScene' : 'dynamic.worldProject') }}</h3>
            <article v-for="system in worldOverviewSystems" :key="system.id" class="world-summary-card"><strong>{{ system.name }}</strong><small>{{ worldCategoryLabel(system.category) }}</small><p>{{ system.principle || system.rules || t("labels.noBriefInfo") }}</p><button type="button" @click="openWorldSystem(system.id)">{{ ui("Öffnen") }}</button></article>
            <p v-if="!worldOverviewSystems.length" class="muted-text">{{ ui("Keine passenden Weltsysteme.") }}</p>
            <template v-if="workspace.activeSection === 'writing' && manuscriptStore.activeSceneId"><h3>{{ ui("Mit Szene verknüpfen") }}</h3><label v-for="system in worldbuildingStore.systems" :key="system.id" class="scene-assignment-row"><input type="checkbox" :checked="sceneWorldSystems.some((item) => item.id === system.id)" @change="toggleSceneWorldSystem(system.id, ($event.target as HTMLInputElement).checked)" /><span>{{ system.name }}</span></label></template>
            <button type="button" class="primary-action" @click="workspace.setActiveSection('worldbuilding')">{{ ui("Zur Worldbuilding-Übersicht") }}</button>
          </section>

          <section v-else class="ai-panel">
            <label>
              <span>{{ ui("Aktion") }}</span>
              <select v-model="sidebarAiAction">
                <option value="summary">{{ ui("Szene zusammenfassen") }}</option>
                <option value="contradictions">{{ ui("Widersprüche prüfen") }}</option>
                <option value="alternatives">{{ ui("Synonyme / Umformulierungen") }}</option>
                <option value="improve">{{ ui("Text verbessern") }}</option>
              </select>
            </label>

            <div class="context-list compact">
              <p v-for="line in aiContextPreview" :key="line">{{ line }}</p>
            </div>

            <template v-if="sidebarAiAction === 'summary'">
              <button
                type="button"
                class="primary-action"
                :disabled="!canRunSceneSummary"
                @click="runSceneSummary"
              >
                {{ t(aiStore.isRunning ? "dynamic.summarising" : "dynamic.run") }}
              </button>
            </template>

            <template v-else-if="sidebarAiAction === 'contradictions'">
              <button
                type="button"
                class="primary-action"
                :disabled="!canRunSceneContradictions"
                @click="runSceneContradictions"
              >
                {{ aiStore.isRunning ? t("dynamic.checking") : ui("Widersprüche prüfen") }}
              </button>
            </template>

            <template v-else-if="sidebarAiAction === 'alternatives'">
              <div class="context-list compact">
                <p v-if="selectedSceneText">{{ ui("Markierung:") }} {{ selectedSceneTextLabel }}</p>
                <p v-else>{{ ui("Markiere ein Wort oder einen Satz im Szenentext.") }}</p>
              </div>

              <button
                type="button"
                class="primary-action"
                :disabled="!canRunTextAlternatives"
                @click="runTextAlternatives"
              >
                {{ t(aiStore.isRunningAlternatives ? "dynamic.findingAlternatives" : "dynamic.createAlternatives") }}
              </button>
            </template>

            <template v-else>
              <div class="context-list compact">
                <p v-if="selectedSceneText">{{ ui("Markierung:") }} {{ selectedSceneTextLabel }}</p>
                <p v-else>{{ ui("Keine Markierung: Ganze Szene wird verbessert.") }}</p>
              </div>

              <button
                type="button"
                class="primary-action"
                :disabled="!canRunTextImprovements"
                @click="runTextImprovements"
              >
                {{ aiStore.isRunningImprovements ? t("dynamic.improving") : ui("Text verbessern") }}
              </button>
            </template>

            <p v-if="aiStore.error" class="error-text">{{ formatUiError(aiStore.error) }}</p>
            <p v-else-if="sidebarAiAction === 'summary' && !sceneDraft.text.trim()" class="muted-text">
              {{ ui("Die Szene braucht Text, bevor sie zusammengefasst werden kann.") }}
            </p>
            <p v-else-if="sidebarAiAction === 'contradictions' && !sceneDraft.text.trim()" class="muted-text">
              {{ ui("Die Szene braucht Text, bevor sie geprüft werden kann.") }}
            </p>
            <p
              v-else-if="sidebarAiAction === 'contradictions' && !writingBibleSceneLinks.length"
              class="muted-text"
            >
              {{ ui("Verknüpfe zuerst mindestens eine Projektbibel-Karte mit dieser Szene.") }}
            </p>
            <p v-else-if="sidebarAiAction === 'alternatives' && !selectedSceneText" class="muted-text">
              {{ ui("Markiere zuerst Text im Szeneneditor.") }}
            </p>
            <p v-else-if="sidebarAiAction === 'improve' && !sceneDraft.text.trim()" class="muted-text">
              {{ ui("Die Szene braucht Text, bevor sie verbessert werden kann.") }}
            </p>
            <p v-else-if="!aiStore.isProviderReachable" class="muted-text">
              {{ ui("Die lokale KI muss erreichbar sein, bevor diese Aktion ausgeführt werden kann.") }}
            </p>
            <p v-else-if="aiModelListEmpty" class="muted-text">
              {{ ui("Es ist noch kein lokales Modell für diese Aktion verfügbar.") }}
            </p>
            <p v-else-if="aiModelMissing" class="muted-text">
              {{ ui("Bitte zuerst ein installiertes Modell auswählen.") }}
            </p>

            <div
              v-if="sidebarAiAction === 'alternatives' && aiStore.textAlternatives.length"
              class="ai-alternatives"
            >
              <article
                v-for="alternative in aiStore.textAlternatives"
                :key="alternative"
                class="ai-suggestion"
              >
                <p>{{ alternative }}</p>
                <div class="ai-suggestion-actions">
                  <button
                    type="button"
                    class="primary-action"
                    @click="applyTextAlternative(alternative)"
                  >
                    {{ ui("Einsetzen") }}
                  </button>
                </div>
              </article>
            </div>

            <div
              v-if="sidebarAiAction === 'improve' && aiStore.textImprovements.length"
              class="ai-alternatives"
            >
              <article
                v-for="improvement in aiStore.textImprovements"
                :key="improvement"
                class="ai-suggestion"
              >
                <p>{{ improvement }}</p>
                <div class="ai-suggestion-actions">
                  <button
                    type="button"
                    class="primary-action"
                    @click="applyTextImprovement(improvement)"
                  >
                    {{ ui("Einsetzen") }}
                  </button>
                </div>
              </article>
            </div>

            <article
              v-if="sidebarAiAction === 'summary' && activeSceneSummarySuggestion"
              class="ai-suggestion"
            >
              <h3>{{ ui("Vorschlag") }}</h3>
              <p>{{ activeSceneSummarySuggestion.result }}</p>
              <small>{{ activeSceneSummarySuggestion.status }}</small>
              <div class="ai-suggestion-actions">
                <button
                  type="button"
                  class="primary-action"
                  :disabled="activeSceneSummarySuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="acceptAiSummary"
                >
                  {{ ui("Übernehmen") }}
                </button>
                <button
                  type="button"
                  class="danger-action"
                  :disabled="activeSceneSummarySuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="rejectSceneAiSuggestion(activeSceneSummarySuggestion.id)"
                >
                  {{ ui("Verwerfen") }}
                </button>
              </div>
            </article>

            <article
              v-if="sidebarAiAction === 'contradictions' && activeSceneContradictionSuggestion"
              class="ai-suggestion"
            >
              <h3>{{ ui("Prüfergebnis") }}</h3>
              <p>{{ activeSceneContradictionSuggestion.result }}</p>
              <small>{{ activeSceneContradictionSuggestion.status }}</small>
              <div class="ai-suggestion-actions single">
                <button
                  type="button"
                  class="danger-action"
                  :disabled="activeSceneContradictionSuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="rejectSceneAiSuggestion(activeSceneContradictionSuggestion.id)"
                >
                  {{ ui("Verwerfen") }}
                </button>
              </div>
            </article>
          </section>
        </template>
      </aside>
</template>
