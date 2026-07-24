import { defineStore } from "pinia";
import { localeCode } from "../i18n";
import { normalizeUiError, type UiError } from "../errors";
import { createWorldSystem, deleteWorldSystem, getWorldbuilding, updateWorldSystem, type WorldbuildingSnapshot, type WorldSystemCategory, type WorldSystemUpdate } from "../api/worldbuilding";

export const worldSystemCategories: Array<{ id: WorldSystemCategory; labelKey: string }> = [
  { id: "magic", labelKey: "worldbuilding.categories.magic" }, { id: "technology", labelKey: "worldbuilding.categories.technology" },
  { id: "psionics", labelKey: "worldbuilding.categories.psionics" }, { id: "biotech", labelKey: "worldbuilding.categories.biotech" },
  { id: "supernatural", labelKey: "worldbuilding.categories.supernatural" }, { id: "hybrid", labelKey: "worldbuilding.categories.hybrid" },
  { id: "other", labelKey: "worldbuilding.categories.other" },
];

export const useWorldbuildingStore = defineStore("worldbuilding", {
  state: () => ({ snapshot: null as WorldbuildingSnapshot | null, selectedSystemId: "", categoryFilter: "all" as WorldSystemCategory | "all", searchQuery: "", isLoading: false, isSaving: false, error: null as UiError | null }),
  getters: {
    systems: (state) => state.snapshot?.systems ?? [],
    templates: (state) => state.snapshot?.templates ?? [],
    filteredSystems(): WorldbuildingSnapshot["systems"] { const query = this.searchQuery.trim().toLocaleLowerCase(localeCode()); return this.systems.filter((system) => (this.categoryFilter === "all" || system.category === this.categoryFilter) && (!query || Object.values(system).join(" ").toLocaleLowerCase(localeCode()).includes(query))); },
    selectedSystem(): WorldbuildingSnapshot["systems"][number] | null { return this.systems.find((system) => system.id === this.selectedSystemId) ?? this.systems[0] ?? null; },
    selectedTemplate(): WorldbuildingSnapshot["templates"][number] | null { const system = this.selectedSystem; return this.templates.find((template) => template.id === system?.templateId) ?? this.templates.find((template) => template.id === "general") ?? null; },
  },
  actions: {
    setSnapshot(snapshot: WorldbuildingSnapshot, preferredId?: string) { this.snapshot = snapshot; const id = preferredId ?? this.selectedSystemId; this.selectedSystemId = snapshot.systems.some((system) => system.id === id) ? id : snapshot.systems[0]?.id ?? ""; },
    async load() { this.isLoading = true; this.error = null; try { this.setSnapshot(await getWorldbuilding()); } catch (error) { this.error = normalizeUiError(error); } finally { this.isLoading = false; } },
    select(id: string) { this.selectedSystemId = id; },
    async create(name: string, category: WorldSystemCategory, templateId: string) { this.isSaving = true; this.error = null; const ids = new Set(this.systems.map((system) => system.id)); try { const snapshot = await createWorldSystem(name, category, templateId); const created = snapshot.systems.find((system) => !ids.has(system.id)); this.setSnapshot(snapshot, created?.id); return created ?? null; } catch (error) { this.error = normalizeUiError(error); return null; } finally { this.isSaving = false; } },
    async update(input: WorldSystemUpdate) { this.isSaving = true; this.error = null; try { this.setSnapshot(await updateWorldSystem(input), input.id); return true; } catch (error) { this.error = normalizeUiError(error); return false; } finally { this.isSaving = false; } },
    async archiveSelected() { if (!this.selectedSystemId) return; this.isSaving = true; this.error = null; try { this.setSnapshot(await deleteWorldSystem(this.selectedSystemId)); } catch (error) { this.error = normalizeUiError(error); } finally { this.isSaving = false; } },
  },
});
