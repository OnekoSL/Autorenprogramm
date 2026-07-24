<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "SearchView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["search"];
    if (guard) navigation.register("search", guard);
    onUnmounted(() => navigation.unregister("search", guard));
    return context;
  },
});
</script>

<template>
<section
        class="editor utility-editor"
        :aria-label="ui('Suche')"
      >
        <div class="utility-panel">
          <p class="eyebrow">{{ ui("Suche") }}</p>
          <h2>{{ ui("Projektweite Suche") }}</h2>
          <label class="search-field">
            <span>{{ ui("Suchbegriff") }}</span>
            <input
              :value="searchStore.query"
              :placeholder="ui('Szenen und Projektbibel durchsuchen')"
              autofocus
              @input="updateSearchQuery"
            />
          </label>
          <div class="search-scope-tabs" :aria-label="ui('Suchbereich')">
            <button
              type="button"
              :class="{ active: searchStore.scope === 'all' }"
              @click="searchStore.setScope('all')"
            >
              {{ ui("Alle") }}
            </button>
            <button
              type="button"
              :class="{ active: searchStore.scope === 'scenes' }"
              @click="searchStore.setScope('scenes')"
            >
              {{ ui("Szenen") }}
            </button>
            <button
              type="button"
              :class="{ active: searchStore.scope === 'bible' }"
              @click="searchStore.setScope('bible')"
            >
              {{ ui("Projektbibel") }}
            </button>
          </div>

          <p v-if="searchStore.error" class="error-text">{{ formatUiError(searchStore.error) }}</p>
          <p v-else-if="searchStore.isLoading" class="muted-text">{{ ui("Suche läuft...") }}</p>
          <p v-else-if="!searchStore.query.trim()" class="muted-text">
            {{ ui("Gib einen Begriff ein, um Szenen und Projektbibel-Karten zu finden.") }}
          </p>
          <p v-else-if="!searchStore.results.length" class="muted-text">
            {{ ui("Keine Treffer gefunden.") }}
          </p>

          <section v-else class="search-results" :aria-label="ui('Suchergebnisse')">
            <article
              v-for="result in searchStore.results"
              :key="result.id"
              class="search-result"
              :class="{ active: result.kind === 'scene' && result.sceneId === manuscriptStore.activeSceneId }"
            >
              <button type="button" @click="selectProjectSearchResult(result)">
                <span class="search-result-kind">
                  {{ searchResultKindLabel(result.kind, result.targetType) }}
                </span>
                <strong>{{ result.title }}</strong>
                <span>{{ result.subtitle }}</span>
              </button>
              <div class="search-result-fields">
                <span v-for="field in result.matchedFields" :key="field">{{ field }}</span>
              </div>
              <p v-for="snippet in result.snippets" :key="snippet">{{ snippet }}</p>
            </article>
          </section>
        </div>
      </section>
</template>
