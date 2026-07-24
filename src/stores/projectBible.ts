import { defineStore } from "pinia";
import { i18n, localeCode } from "../i18n";
import { normalizeUiError, type UiError } from "../errors";
import {
  createProjectBibleCard,
  deleteProjectBibleCard,
  getProjectBible,
  replaceSceneLinksForCard,
  updateProjectBibleCard,
  upsertBibleRelationship,
  deleteBibleRelationship,
  reorderBibleEvents,
  type CharacterSnapshot,
  type ItemSnapshot,
  type LocationSnapshot,
  type NoteSnapshot,
  type FactionSnapshot,
  type EventSnapshot,
  type BibleRelationshipUpdate,
  type NoteType,
  type ProjectBibleCardType,
  type KnowledgeTargetType,
  type ProjectBibleCardUpdate,
  type ProjectBibleSnapshot,
  type SceneLinkSnapshot,
} from "../api/projectBible";
import type { WorldSystemSnapshot } from "../api/worldbuilding";

export type ProjectBibleCard =
  | (CharacterSnapshot & { targetType: "character"; title: string; subtitle: string })
  | (LocationSnapshot & { targetType: "location"; title: string; subtitle: string })
  | (ItemSnapshot & { targetType: "item"; title: string; subtitle: string })
  | (NoteSnapshot & { targetType: "note"; name: string; subtitle: string })
  | (FactionSnapshot & { targetType: "faction"; title: string; subtitle: string })
  | (EventSnapshot & { targetType: "event"; name: string; subtitle: string });

export type KnowledgeCard = ProjectBibleCard
  | (WorldSystemSnapshot & { targetType: "world_system"; title: string; subtitle: string });

export const projectBibleCardTypes: Array<{ id: ProjectBibleCardType; labelKey: string }> = [
  { id: "character", labelKey: "bible.types.character" },
  { id: "location", labelKey: "bible.types.location" },
  { id: "item", labelKey: "bible.types.item" },
  { id: "note", labelKey: "bible.types.note" },
  { id: "faction", labelKey: "bible.types.faction" },
  { id: "event", labelKey: "bible.types.event" },
];

export const noteTypes: Array<{ id: NoteType; labelKey: string }> = [
  { id: "general", labelKey: "bible.noteTypes.general" },
  { id: "research", labelKey: "bible.noteTypes.research" },
  { id: "lore", labelKey: "bible.noteTypes.lore" },
  { id: "todo", labelKey: "bible.noteTypes.todo" },
];

function cardsForType(
  snapshot: ProjectBibleSnapshot | null,
  type: ProjectBibleCardType,
): ProjectBibleCard[] {
  if (!snapshot) {
    return [];
  }

  if (type === "character") {
    return snapshot.characters.map((card) => ({
      ...card,
      targetType: "character",
      title: card.name,
      subtitle: card.role,
    }));
  }

  if (type === "location") {
    return snapshot.locations.map((card) => ({
      ...card,
      targetType: "location",
      title: card.name,
      subtitle: card.description,
    }));
  }

  if (type === "item") {
    return snapshot.items.map((card) => ({
      ...card,
      targetType: "item",
      title: card.name,
      subtitle: card.description,
    }));
  }

  if (type === "faction") return snapshot.factions.map((card) => ({ ...card, targetType: "faction", title: card.name, subtitle: card.factionType || card.description }));
  if (type === "event") return snapshot.events.map((card) => ({ ...card, targetType: "event", name: card.title, subtitle: card.timeLabel || card.summary }));
  return snapshot.notes.map((card) => ({
    ...card,
    targetType: "note",
    name: card.title,
    subtitle: i18n.global.t(noteTypes.find((noteType) => noteType.id === card.noteType)?.labelKey ?? card.noteType),
  }));
}

function allCardIds(snapshot: ProjectBibleSnapshot | null, type: ProjectBibleCardType) {
  return new Set(cardsForType(snapshot, type).map((card) => card.id));
}

export const useProjectBibleStore = defineStore("projectBible", {
  state: () => ({
    snapshot: null as ProjectBibleSnapshot | null,
    activeCardType: "character" as ProjectBibleCardType,
    selectedCardId: "",
    searchQuery: "",
    isLoading: false,
    isSaving: false,
    error: null as UiError | null,
  }),
  getters: {
    cards(state): ProjectBibleCard[] {
      return cardsForType(state.snapshot, state.activeCardType);
    },
    filteredCards(): ProjectBibleCard[] {
      const query = this.searchQuery.trim().toLocaleLowerCase(localeCode());
      if (!query) {
        return this.cards;
      }

      return this.cards.filter((card) =>
        [card.title, card.subtitle, "description" in card ? card.description : "", "body" in card ? card.body : "", "goals" in card ? card.goals : "", "summary" in card ? card.summary : "", "outcome" in card ? card.outcome : "", "notes" in card ? card.notes : ""]
          .join(" ")
          .toLocaleLowerCase(localeCode())
          .includes(query),
      );
    },
    selectedCard(): ProjectBibleCard | null {
      return this.cards.find((card) => card.id === this.selectedCardId) ?? this.cards[0] ?? null;
    },
    selectedCardLinks(state): SceneLinkSnapshot[] {
      if (!state.snapshot || !state.selectedCardId) {
        return [];
      }

      return state.snapshot.sceneLinks.filter(
        (link) =>
          link.targetType === state.activeCardType && link.targetId === state.selectedCardId,
      );
    },
    selectedCardRelationships(state) {
      if (!state.snapshot || !state.selectedCardId) return [];
      return state.snapshot.relationships.filter((item) => (item.sourceType === state.activeCardType && item.sourceId === state.selectedCardId) || (item.targetType === state.activeCardType && item.targetId === state.selectedCardId));
    },
  },
  actions: {
    reset() {
      this.snapshot = null;
      this.selectedCardId = "";
      this.searchQuery = "";
      this.error = null;
    },
    setSnapshot(snapshot: ProjectBibleSnapshot, preferredCardId?: string) {
      this.snapshot = snapshot;
      const cards = cardsForType(snapshot, this.activeCardType);
      const selectedCardId = preferredCardId ?? this.selectedCardId;
      this.selectedCardId = cards.some((card) => card.id === selectedCardId)
        ? selectedCardId
        : cards[0]?.id ?? "";
    },
    setActiveCardType(type: ProjectBibleCardType) {
      this.activeCardType = type;
      const cards = cardsForType(this.snapshot, type);
      this.selectedCardId = cards.some((card) => card.id === this.selectedCardId)
        ? this.selectedCardId
        : cards[0]?.id ?? "";
    },
    selectCard(cardId: string) {
      this.selectedCardId = cardId;
    },
    async load() {
      this.isLoading = true;
      this.error = null;

      try {
        this.setSnapshot(await getProjectBible());
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async createCard(type?: ProjectBibleCardType) {
      const targetType = type ?? this.activeCardType;
      const beforeIds = allCardIds(this.snapshot, targetType);
      this.activeCardType = targetType;
      this.isSaving = true;
      this.error = null;

      try {
        const snapshot = await createProjectBibleCard(targetType);
        const newCard = cardsForType(snapshot, targetType).find((card) => !beforeIds.has(card.id));
        this.setSnapshot(snapshot, newCard?.id);
        return newCard ?? null;
      } catch (error) {
        this.error = normalizeUiError(error);
        return null;
      } finally {
        this.isSaving = false;
      }
    },
    async updateCard(input: ProjectBibleCardUpdate) {
      this.isSaving = true;
      this.error = null;

      try {
        this.setSnapshot(await updateProjectBibleCard(input), input.id);
        return true;
      } catch (error) {
        this.error = normalizeUiError(error);
        return false;
      } finally {
        this.isSaving = false;
      }
    },
    async deleteSelectedCard() {
      const card = this.selectedCard;
      if (!card) {
        return;
      }

      this.isSaving = true;
      this.error = null;

      try {
        this.setSnapshot(await deleteProjectBibleCard(card.targetType, card.id));
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isSaving = false;
      }
    },
    async replaceSelectedCardSceneLinks(sceneIds: string[]) {
      const card = this.selectedCard;
      if (!card) {
        return;
      }

      this.isSaving = true;
      this.error = null;

      try {
        this.setSnapshot(await replaceSceneLinksForCard(card.targetType, card.id, sceneIds), card.id);
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isSaving = false;
      }
    },
    async addSceneLinkToCard(targetType: KnowledgeTargetType, targetId: string, sceneId: string) {
      if (!sceneId) {
        return;
      }

      const sceneIds = new Set(
        this.snapshot?.sceneLinks
          .filter((link) => link.targetType === targetType && link.targetId === targetId)
          .map((link) => link.sceneId) ?? [],
      );
      sceneIds.add(sceneId);
      if (targetType !== "world_system") {
        this.activeCardType = targetType;
        this.selectedCardId = targetId;
      }
      this.isSaving = true;
      this.error = null;

      try {
        this.setSnapshot(await replaceSceneLinksForCard(targetType, targetId, [...sceneIds]), targetId);
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isSaving = false;
      }
    },
    async replaceCardSceneLinks(targetType: KnowledgeTargetType, targetId: string, sceneIds: string[]) {
      this.isSaving = true; this.error = null;
      try { this.setSnapshot(await replaceSceneLinksForCard(targetType, targetId, sceneIds)); }
      catch (error) { this.error = normalizeUiError(error); }
      finally { this.isSaving = false; }
    },
    async saveRelationship(input: BibleRelationshipUpdate) {
      this.isSaving = true; this.error = null;
      try { this.setSnapshot(await upsertBibleRelationship(input), this.selectedCardId); return true; }
      catch (error) { this.error = normalizeUiError(error); return false; }
      finally { this.isSaving = false; }
    },
    async removeRelationship(id: string) {
      this.isSaving = true; this.error = null;
      try { this.setSnapshot(await deleteBibleRelationship(id), this.selectedCardId); }
      catch (error) { this.error = normalizeUiError(error); }
      finally { this.isSaving = false; }
    },
    async reorderEvents(eventIds: string[]) {
      this.isSaving = true; this.error = null;
      try { this.setSnapshot(await reorderBibleEvents(eventIds), this.selectedCardId); }
      catch (error) { this.error = normalizeUiError(error); }
      finally { this.isSaving = false; }
    },
  },
});
