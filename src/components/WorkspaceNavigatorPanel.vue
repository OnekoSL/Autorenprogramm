<script lang="ts">
import { defineComponent } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";

export default defineComponent({
  name: "WorkspaceNavigatorPanel",
  setup() {
    return useWorkspaceViewContext();
  },
});
</script>

<template>
<aside
        v-if="workspace.activeSection !== 'bible' && workspace.activeSection !== 'structure' && workspace.activeSection !== 'worldbuilding'"
        class="navigator"
        :aria-label="ui('Kapitel und Szenen')"
      >
        <div class="panel-heading">
          <h2>{{ ui("Navigator") }}</h2>
          <span>{{ t("counts.areas", manuscriptStore.areas.length) }}</span>
        </div>

        <p v-if="manuscriptStore.error" class="error-text">{{ formatUiError(manuscriptStore.error) }}</p>
        <p v-if="manuscriptStore.isLoading" class="muted-text">{{ ui("Lädt Manuskript") }}</p>

        <ol v-else class="area-list">
          <li
            v-for="(area, areaIndex) in manuscriptStore.areas"
            :key="area.id"
            class="area-node"
            :class="{ 'drop-target': pointerDropKey === areaDropKey(area.id) }"
            data-drop-kind="area"
            :data-area-id="area.id"
            :data-area-index="areaIndex"
          >
            <div class="area-row">
              <button
                type="button"
                class="drag-handle"
                :aria-label="ui('Bereich ziehen')"
                :title="ui('Bereich ziehen')"
                @pointerdown.stop="startAreaPointerDrag(area, $event)"
              >
                ::
              </button>
              <button
                type="button"
                class="collapse-toggle"
                :aria-expanded="!isAreaCollapsed(area.id)"
                :aria-label="t(isAreaCollapsed(area.id) ? 'dynamic.expandArea' : 'dynamic.collapseArea')"
                :title="t(isAreaCollapsed(area.id) ? 'dynamic.expandArea' : 'dynamic.collapseArea')"
                @click.stop="toggleAreaCollapsed(area.id)"
              >
                {{ isAreaCollapsed(area.id) ? ">" : "v" }}
              </button>
              <input
                class="area-title navigator-title-input"
                :class="{ 'navigator-focused': focusedNavigatorKey === areaNavigatorKey(area.id) }"
                :value="area.title"
                :data-navigator-key="areaNavigatorKey(area.id)"
                :aria-label="ui('Bereichsname')"
                @focus="
                  selectArea(area);
                  focusNavigatorItem(areaNavigatorKey(area.id));
                "
                @blur="blurNavigatorItem(areaNavigatorKey(area.id))"
                @change="updateAreaTitle(area, $event)"
                @keydown.enter.prevent="commitChapterTitle"
                @keydown="handleAreaKeydown(area, $event)"
              />
              <button
                type="button"
                class="icon-action"
                :aria-label="ui('Bereich archivieren')"
                :title="ui('Bereich archivieren')"
                @click="deleteArea(area)"
              >
                {{ ui("x") }}
              </button>
            </div>

            <ol
              v-if="!isAreaCollapsed(area.id)"
              class="chapter-list"
              data-drop-kind="area-end"
              :data-area-id="area.id"
            >
              <li
                v-for="(chapter, chapterIndex) in area.chapters"
                :key="chapter.id"
                class="chapter-node"
                :class="{ 'drop-target': pointerDropKey === chapterDropKey(chapter.id) }"
                data-drop-kind="chapter"
                :data-area-id="area.id"
                :data-chapter-id="chapter.id"
                :data-chapter-index="chapterIndex"
              >
                <div class="chapter-row">
                  <button
                    type="button"
                    class="drag-handle"
                    :aria-label="ui('Kapitel ziehen')"
                    :title="ui('Kapitel ziehen')"
                    @pointerdown.stop="startChapterPointerDrag(chapter, $event)"
                  >
                    ::
                  </button>
                  <button
                    type="button"
                    class="collapse-toggle"
                    :aria-expanded="!isChapterCollapsed(chapter.id)"
                    :aria-label="
                      t(isChapterCollapsed(chapter.id) ? 'dynamic.expandChapter' : 'dynamic.collapseChapter')
                    "
                    :title="
                      t(isChapterCollapsed(chapter.id) ? 'dynamic.expandChapter' : 'dynamic.collapseChapter')
                    "
                    @click.stop="toggleChapterCollapsed(chapter.id)"
                  >
                    {{ isChapterCollapsed(chapter.id) ? ">" : "v" }}
                  </button>
                  <input
                    class="chapter-title navigator-title-input"
                    :class="{ 'navigator-focused': focusedNavigatorKey === chapterNavigatorKey(chapter.id) }"
                    :value="chapter.title"
                    :data-navigator-key="chapterNavigatorKey(chapter.id)"
                    :aria-label="ui('Kapitelname')"
                    @focus="
                      selectChapter(chapter);
                      focusNavigatorItem(chapterNavigatorKey(chapter.id));
                    "
                    @blur="blurNavigatorItem(chapterNavigatorKey(chapter.id))"
                    @change="updateChapterTitle(chapter, $event)"
                    @keydown.enter.prevent="commitChapterTitle"
                    @keydown="handleChapterKeydownInArea(area, chapter, $event)"
                  />
                  <button
                    type="button"
                    class="icon-action"
                    :aria-label="ui('Kapitel archivieren')"
                    :title="ui('Kapitel archivieren')"
                    @click="deleteChapter(chapter)"
                  >
                    {{ ui("x") }}
                  </button>
                </div>

                <ol
                  v-if="!isChapterCollapsed(chapter.id)"
                  class="scene-list"
                  data-drop-kind="chapter-end"
                  :data-chapter-id="chapter.id"
                >
                  <li
                    v-for="(scene, sceneIndex) in chapter.scenes"
                    :key="scene.id"
                    class="scene-row"
                    :class="{
                      selected: manuscriptStore.activeSceneId === scene.id,
                      'navigator-focused': focusedNavigatorKey === sceneNavigatorKey(scene.id),
                      'drop-target': pointerDropKey === sceneDropKey(chapter.id, sceneIndex),
                    }"
                    data-drop-kind="scene"
                    :data-chapter-id="chapter.id"
                    :data-scene-index="sceneIndex"
                  >
                    <button
                      type="button"
                      class="drag-handle"
                      :aria-label="ui('Szene ziehen')"
                      :title="ui('Szene ziehen')"
                      @pointerdown.stop="startScenePointerDrag(chapter, scene, $event)"
                    >
                      ::
                    </button>
                    <button
                      type="button"
                      class="scene-title"
                      :data-navigator-key="sceneNavigatorKey(scene.id)"
                      @focus="
                        void selectScene(scene.id);
                        focusNavigatorItem(sceneNavigatorKey(scene.id));
                      "
                      @blur="blurNavigatorItem(sceneNavigatorKey(scene.id))"
                      @click="selectScene(scene.id)"
                      @keydown="handleSceneKeydown(chapter, scene, $event)"
                    >
                      {{ scene.title }}
                    </button>
                    <button
                      type="button"
                      class="icon-action"
                      :aria-label="ui('Szene archivieren')"
                      :title="ui('Szene archivieren')"
                      @click="deleteScene(scene)"
                    >
                      {{ ui("x") }}
                    </button>
                  </li>
                  <li
                    class="scene-drop-target"
                    :class="{ 'drop-target': pointerDropKey === chapterEndDropKey(chapter.id) }"
                    data-drop-kind="chapter-end"
                    :data-chapter-id="chapter.id"
                  >
                    <button type="button" @click="createSceneFromNavigator(chapter.id)">{{ ui("Szene anlegen") }}</button>
                  </li>
                </ol>
              </li>
              <li
                class="chapter-drop-target"
                :class="{ 'drop-target': pointerDropKey === areaEndDropKey(area.id) }"
                data-drop-kind="area-end"
                :data-area-id="area.id"
              >
                <button type="button" @click="createChapterFromNavigator(area.id)">{{ ui("Kapitel anlegen") }}</button>
              </li>
            </ol>
          </li>
        </ol>

        <div class="navigator-actions">
          <button type="button" @click="createAreaFromNavigator">{{ ui("Bereich") }}</button>
          <button type="button" @click="createChapterFromNavigator()">{{ ui("Kapitel") }}</button>
          <button type="button" @click="createSceneForActiveChapter">{{ ui("Szene") }}</button>
        </div>
      </aside>

<aside
        v-else-if="workspace.activeSection === 'structure'"
        class="navigator structure-navigator"
        :aria-label="ui('Strukturvorlagen')"
      >
        <div class="panel-heading">
          <h2>{{ ui("Strukturmodelle") }}</h2>
          <span>{{ t("counts.templates", storyStructureStore.templates.length) }}</span>
        </div>

        <p v-if="storyStructureStore.error" class="error-text">{{ formatUiError(storyStructureStore.error) }}</p>
        <p v-else-if="storyStructureStore.isLoading" class="muted-text">{{ ui("Lädt Vorlagen") }}</p>

        <ol v-else class="structure-template-list">
          <li v-for="template in storyStructureStore.templates" :key="template.id">
            <button
              type="button"
              :class="{
                selected: storyStructureStore.selectedTemplateId === template.id,
                applied: storyStructureStore.activeStructure?.templateId === template.id,
              }"
              @click="selectStructureTemplate(template.id)"
            >
              <span class="structure-template-heading">
                <strong>{{ template.name }}</strong>
                <small v-if="storyStructureStore.activeStructure?.templateId === template.id">{{ ui("Aktiv") }}</small>
              </span>
              <span>{{ structureCategoryLabels[template.category] }}</span>
              <small>{{ t("counts.acts", template.acts.length) }}</small>
            </button>
          </li>
        </ol>

        <div class="navigator-actions structure-actions">
          <button
            type="button"
            class="primary-action"
            :disabled="storyStructureStore.isApplying || !structureIsPreview"
            @click="applySelectedStructureTemplate"
          >
            {{ t(storyStructureStore.activeStructure ? "dynamic.switchTemplate" : "dynamic.applyTemplate") }}
          </button>
          <small v-if="!structureIsPreview">{{ ui("Diese Vorlage ist aktiv.") }}</small>
          <button
            v-if="storyStructureStore.activeStructure && !storyStructureStore.hasGeneratedManuscript && !structureIsPreview"
            type="button"
            class="primary-action"
            :disabled="storyStructureStore.isApplying"
            @click="generateActiveStructureManuscript"
          >
            {{ ui("Gliederung erzeugen") }}
          </button>
        </div>
      </aside>

<aside v-else-if="workspace.activeSection === 'worldbuilding'" class="navigator bible-navigator" :aria-label="ui('Weltsysteme')">
        <div class="panel-heading"><h2>{{ ui("Worldbuilding") }}</h2><span>{{ t("counts.systems", worldbuildingStore.systems.length) }}</span></div>
        <p v-if="worldbuildingStore.error" class="error-text">{{ formatUiError(worldbuildingStore.error) }}</p>
        <select v-model="worldbuildingStore.categoryFilter"><option value="all">{{ ui("Alle Kategorien") }}</option><option v-for="category in worldSystemCategories" :key="category.id" :value="category.id">{{ category.label }}</option></select>
        <label class="navigator-search"><span>{{ ui("Suche") }}</span><input v-model="worldbuildingStore.searchQuery" :placeholder="ui('Weltsystem suchen')" /></label>
        <ol class="bible-card-list"><li v-for="system in worldbuildingStore.filteredSystems" :key="system.id" class="bible-card-row" :class="{ selected: system.id === worldbuildingStore.selectedSystemId }"><button type="button" @click="selectWorldSystem(system.id)"><strong>{{ system.name }}</strong><span>{{ worldCategoryLabel(system.category) }}</span></button></li></ol>
        <section v-if="!worldbuildingStore.filteredSystems.length" class="empty-panel compact-empty"><p>{{ ui("Noch keine Weltsysteme.") }}</p></section>
        <div class="navigator-actions bible-actions"><button type="button" @click="openNewWorldSystemDialog">{{ ui("Neues Weltsystem") }}</button></div>
      </aside>

<aside v-else class="navigator bible-navigator" :aria-label="ui('Projektbibel-Karten')">
        <div class="panel-heading">
          <h2>{{ ui("Projektbibel") }}</h2>
          <span>{{ t("counts.cards", projectBibleStore.cards.length) }}</span>
        </div>

        <p v-if="projectBibleStore.error" class="error-text">{{ formatUiError(projectBibleStore.error) }}</p>
        <p v-else-if="projectBibleStore.isLoading" class="muted-text">{{ ui("Lädt Projektbibel") }}</p>

        <div class="bible-type-list" :aria-label="ui('Kartentypen')">
          <button
            v-for="cardType in projectBibleCardTypes"
            :key="cardType.id"
            type="button"
            :class="{ active: projectBibleStore.activeCardType === cardType.id }"
            @click="selectBibleCardType(cardType.id)"
          >
            {{ cardType.label }}
          </button>
        </div>

        <label class="navigator-search">
          <span>{{ ui("Suche") }}</span>
          <input v-model="projectBibleStore.searchQuery" :placeholder="ui('Karte suchen')" />
        </label>

        <ol class="bible-card-list">
          <li
            v-for="card in projectBibleStore.filteredCards"
            :key="card.id"
            class="bible-card-row"
            :draggable="projectBibleStore.activeCardType === 'event'"
            @dragstart="draggedEventId = card.id"
            @dragover.prevent
            @drop="dropBibleEvent(card.id)"
            :class="{
              selected: selectedCard?.id === card.id,
              'navigator-focused': focusedNavigatorKey === bibleCardNavigatorKey(card.id),
            }"
          >
            <button
              type="button"
              :data-navigator-key="bibleCardNavigatorKey(card.id)"
              @focus="selectBibleCard(card)"
              @blur="blurNavigatorItem(bibleCardNavigatorKey(card.id))"
              @click="selectBibleCard(card)"
            >
              <strong>{{ card.title }}</strong>
              <span>{{ card.subtitle || cardTypeSingular(card.targetType) }}</span>
            </button>
          </li>
        </ol>

        <section v-if="!projectBibleStore.filteredCards.length" class="empty-panel compact-empty">
          <p>{{ ui("Keine Karten gefunden.") }}</p>
        </section>

        <div class="navigator-actions bible-actions">
          <button type="button" @click="createBibleCard">{{ ui("Neue Karte") }}</button>
        </div>
      </aside>

<button
        type="button"
        class="resize-handle"
        :aria-label="ui('Navigator verbreitern oder verkleinern')"
        @pointerdown="startPanelResize('navigator', $event)"
      />
</template>
