<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "WorldbuildingView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["worldbuilding"];
    if (guard) navigation.register("worldbuilding", guard);
    onUnmounted(() => navigation.unregister("worldbuilding", guard));
    return context;
  },
});
</script>

<template>
<section
        class="editor bible-editor worldbuilding-editor"
        :aria-label="ui('Worldbuilding')"
      >
        <template v-if="worldbuildingStore.selectedSystem">
          <div class="card-meta"><div><p class="eyebrow">{{ worldCategoryLabel(worldSystemDraft.category) }}</p><h2>{{ worldSystemDraft.name }}</h2></div><button type="button" class="danger-action" @click="archiveWorldSystem">{{ ui("Archivieren") }}</button></div>
          <div class="bible-form card-editor-form world-system-form">
            <label><span>{{ worldSystemFieldLabels.name }}</span><input v-model="worldSystemDraft.name" /></label>
            <label><span>{{ ui("Kategorie") }}</span><select v-model="worldSystemDraft.category"><option v-for="category in worldSystemCategories" :key="category.id" :value="category.id">{{ category.label }}</option></select></label>
            <label><span>{{ ui("Vorlage") }}</span><select v-model="worldSystemDraft.templateId"><option v-for="template in worldbuildingStore.templates" :key="template.id" :value="template.id">{{ template.name }}</option></select></label>
            <label><span>{{ worldSystemFieldLabels.principle }}</span><textarea v-model="worldSystemDraft.principle" /></label>
            <label><span>{{ worldSystemFieldLabels.rules }}</span><textarea v-model="worldSystemDraft.rules" /></label>
            <label><span>{{ worldSystemFieldLabels.capabilities }}</span><textarea v-model="worldSystemDraft.capabilities" /></label>
            <label><span>{{ worldSystemFieldLabels.costs }}</span><textarea v-model="worldSystemDraft.costs" /></label>
            <label><span>{{ worldSystemFieldLabels.limits }}</span><textarea v-model="worldSystemDraft.limits" /></label>
            <label><span>{{ worldSystemFieldLabels.access }}</span><textarea v-model="worldSystemDraft.access" /></label>
            <label><span>{{ worldSystemFieldLabels.distribution }}</span><textarea v-model="worldSystemDraft.distribution" /></label>
            <label><span>{{ worldSystemFieldLabels.worldImpact }}</span><textarea v-model="worldSystemDraft.worldImpact" /></label>
            <label><span>{{ worldSystemFieldLabels.openQuestions }}</span><textarea v-model="worldSystemDraft.openQuestions" /></label>
            <label><span>{{ worldSystemFieldLabels.notes }}</span><textarea v-model="worldSystemDraft.notes" /></label>
          </div>
        </template>
        <section v-else class="empty-editor"><h2>{{ ui("Noch kein Weltsystem") }}</h2><p>{{ ui("Lege ein System aus einer Vorlage an.") }}</p><button type="button" class="primary-action" @click="openNewWorldSystemDialog">{{ ui("Weltsystem anlegen") }}</button></section>
      </section>

<div v-if="showWorldSystemDialog" class="modal-backdrop" @click.self="showWorldSystemDialog = false"><form class="modal-card" @submit.prevent="createWorldSystemFromDialog"><h2>{{ ui("Neues Weltsystem") }}</h2><label><span>{{ ui("Name") }}</span><input v-model="newWorldSystemName" autofocus /></label><label><span>{{ ui("Kategorie") }}</span><select v-model="newWorldSystemCategory"><option v-for="category in worldSystemCategories" :key="category.id" :value="category.id">{{ category.label }}</option></select></label><label><span>{{ ui("Vorlage") }}</span><select v-model="newWorldSystemTemplateId"><option v-for="template in worldbuildingStore.templates" :key="template.id" :value="template.id">{{ template.name }}</option></select></label><div class="modal-actions"><button type="button" @click="showWorldSystemDialog = false">{{ ui("Abbrechen") }}</button><button type="submit" class="primary-action" :disabled="!newWorldSystemName.trim() || worldbuildingStore.isSaving">{{ ui("Anlegen") }}</button></div></form></div>
</template>
