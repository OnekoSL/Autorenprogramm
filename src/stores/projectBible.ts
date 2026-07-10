import { defineStore } from "pinia";
import {
  createProjectBibleCard,
  deleteProjectBibleCard,
  getProjectBible,
  replaceSceneLinksForCard,
  updateProjectBibleCard,
  type CharacterSnapshot,
  type ItemSnapshot,
  type LocationSnapshot,
  type NoteSnapshot,
  type NoteType,
  type ProjectBibleCardType,
  type ProjectBibleCardUpdate,
  type ProjectBibleSnapshot,
  type SceneLinkSnapshot,
} from "../api/projectBible";
import type { CommandError } from "../api/project";

export type ProjectBibleCard =
  | (CharacterSnapshot & { targetType: "character"; title: string; subtitle: string })
  | (LocationSnapshot & { targetType: "location"; title: string; subtitle: string })
  | (ItemSnapshot & { targetType: "item"; title: string; subtitle: string })
  | (NoteSnapshot & { targetType: "note"; name: string; subtitle: string });

export const projectBibleCardTypes: Array<{ id: ProjectBibleCardType; label: string }> = [
  { id: "character", label: "Figuren" },
  { id: "location", label: "Orte" },
  { id: "item", label: "Gegenstaende" },
  { id: "note", label: "Notizen" },
];

export const noteTypes: Array<{ id: NoteType; label: string }> = [
  { id: "general", label: "Allgemein" },
  { id: "research", label: "Recherche" },
  { id: "lore", label: "Lore" },
  { id: "faction", label: "Fraktion" },
  { id: "event", label: "Ereignis" },
  { id: "todo", label: "Todo" },
];

function normalizeError(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return (error as CommandError).message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Die Projektbibel-Aktion ist fehlgeschlagen.";
}

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

  return snapshot.notes.map((card) => ({
    ...card,
    targetType: "note",
    name: card.title,
    subtitle: noteTypes.find((noteType) => noteType.id === card.noteType)?.label ?? card.noteType,
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
    error: "",
  }),
  getters: {
    cards(state): ProjectBibleCard[] {
      return cardsForType(state.snapshot, state.activeCardType);
    },
    filteredCards(): ProjectBibleCard[] {
      const query = this.searchQuery.trim().toLocaleLowerCase();
      if (!query) {
        return this.cards;
      }

      return this.cards.filter((card) =>
        [card.title, card.subtitle, "description" in card ? card.description : "", "body" in card ? card.body : ""]
          .join(" ")
          .toLocaleLowerCase()
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
  },
  actions: {
    reset() {
      this.snapshot = null;
      this.selectedCardId = "";
      this.searchQuery = "";
      this.error = "";
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
      this.error = "";

      try {
        this.setSnapshot(await getProjectBible());
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async createCard(type?: ProjectBibleCardType) {
      const targetType = type ?? this.activeCardType;
      const beforeIds = allCardIds(this.snapshot, targetType);
      this.activeCardType = targetType;
      this.isSaving = true;
      this.error = "";

      try {
        const snapshot = await createProjectBibleCard(targetType);
        const newCard = cardsForType(snapshot, targetType).find((card) => !beforeIds.has(card.id));
        this.setSnapshot(snapshot, newCard?.id);
        return newCard ?? null;
      } catch (error) {
        this.error = normalizeError(error);
        return null;
      } finally {
        this.isSaving = false;
      }
    },
    async updateCard(input: ProjectBibleCardUpdate) {
      this.isSaving = true;
      this.error = "";

      try {
        this.setSnapshot(await updateProjectBibleCard(input), input.id);
      } catch (error) {
        this.error = normalizeError(error);
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
      this.error = "";

      try {
        this.setSnapshot(await deleteProjectBibleCard(card.targetType, card.id));
      } catch (error) {
        this.error = normalizeError(error);
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
      this.error = "";

      try {
        this.setSnapshot(await replaceSceneLinksForCard(card.targetType, card.id, sceneIds), card.id);
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isSaving = false;
      }
    },
    async addSceneLinkToCard(targetType: ProjectBibleCardType, targetId: string, sceneId: string) {
      if (!sceneId) {
        return;
      }

      const sceneIds = new Set(
        this.snapshot?.sceneLinks
          .filter((link) => link.targetType === targetType && link.targetId === targetId)
          .map((link) => link.sceneId) ?? [],
      );
      sceneIds.add(sceneId);
      this.activeCardType = targetType;
      this.selectedCardId = targetId;
      this.isSaving = true;
      this.error = "";

      try {
        this.setSnapshot(await replaceSceneLinksForCard(targetType, targetId, [...sceneIds]), targetId);
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isSaving = false;
      }
    },
  },
});
