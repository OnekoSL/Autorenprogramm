import { defineStore } from "pinia";
import { normalizeUiError, type UiError } from "../errors";
import {
  acceptAiSuggestion,
  acceptAiBibleSuggestion,
  checkSceneContradictions,
  completeBibleCardFromKeywords,
  editBibleCard,
  generateBibleCardFromKeywords,
  generateTextAlternatives,
  getAiProviderStatus,
  getAiSettings,
  improveText,
  listAiBibleSuggestions,
  listAiModels,
  listAiSuggestionsForScene,
  rejectAiBibleSuggestion,
  rejectAiSuggestion,
  sendAiChatMessage,
  startLocalAiProvider,
  summarizeScene,
  updateAiSettings,
  type AiBibleAcceptResult,
  type AiBibleCardDraft,
  type AiBibleEditableField,
  type AiBibleSuggestionSnapshot,
  type AiChatMode,
  type AiChatMessage,
  type GenerateTextAlternativesInput,
  type ImproveTextInput,
  type AiModelInfo,
  type AiProviderStatus,
  type AiSettings,
  type AiSuggestionSnapshot,
  type UpdateAiSettingsInput,
} from "../api/ai";
import type { ProjectBibleCardType } from "../api/projectBible";

export const useAiStore = defineStore("ai", {
  state: () => ({
    settings: null as AiSettings | null,
    providerStatus: null as AiProviderStatus | null,
    models: [] as AiModelInfo[],
    chatMessages: [] as AiChatMessage[],
    textAlternatives: [] as string[],
    textImprovements: [] as string[],
    suggestions: [] as AiSuggestionSnapshot[],
    bibleSuggestions: [] as AiBibleSuggestionSnapshot[],
    activeSuggestionId: "",
    activeBibleSuggestionId: "",
    isLoading: false,
    isLoadingBibleSuggestions: false,
    isLoadingModels: false,
    isCheckingProvider: false,
    isStartingProvider: false,
    isRunning: false,
    isRunningBible: false,
    isRunningChat: false,
    isRunningAlternatives: false,
    isRunningImprovements: false,
    isSaving: false,
    error: null as UiError | null,
  }),
  getters: {
    activeSuggestion(state): AiSuggestionSnapshot | null {
      return (
        state.suggestions.find((suggestion) => suggestion.id === state.activeSuggestionId) ??
        state.suggestions.find((suggestion) => suggestion.status === "draft") ??
        state.suggestions[0] ??
        null
      );
    },
    draftSuggestion(): AiSuggestionSnapshot | null {
      return this.suggestions.find((suggestion) => suggestion.status === "draft") ?? null;
    },
    activeBibleSuggestion(state): AiBibleSuggestionSnapshot | null {
      return (
        state.bibleSuggestions.find((suggestion) => suggestion.id === state.activeBibleSuggestionId) ??
        state.bibleSuggestions.find((suggestion) => suggestion.status === "draft") ??
        state.bibleSuggestions[0] ??
        null
      );
    },
    draftBibleSuggestion(): AiBibleSuggestionSnapshot | null {
      return this.bibleSuggestions.find((suggestion) => suggestion.status === "draft") ?? null;
    },
    isProviderReachable(state): boolean {
      return state.providerStatus?.state === "reachable";
    },
    hasModels(state): boolean {
      return state.models.length > 0;
    },
    isConfiguredModelAvailable(state): boolean {
      const model = state.settings?.model ?? "";
      return Boolean(model && state.models.some((item) => item.name === model || item.id === model));
    },
  },
  actions: {
    reset() {
      this.suggestions = [];
      this.activeSuggestionId = "";
      this.bibleSuggestions = [];
      this.activeBibleSuggestionId = "";
      this.chatMessages = [];
      this.textAlternatives = [];
      this.textImprovements = [];
      this.error = null;
    },
    clearChat() {
      this.chatMessages = [];
      this.error = null;
    },
    clearTextAlternatives() {
      this.textAlternatives = [];
    },
    clearTextImprovements() {
      this.textImprovements = [];
    },
    async loadSettings() {
      this.error = null;

      try {
        this.settings = await getAiSettings();
        await this.checkProviderStatus();
        if (this.isProviderReachable) {
          await this.loadModels();
        }
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async updateSettings(input: UpdateAiSettingsInput) {
      this.isSaving = true;
      this.error = null;

      try {
        this.settings = await updateAiSettings(input);
        await this.checkProviderStatus();
        if (this.isProviderReachable) {
          await this.loadModels();
        }
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isSaving = false;
      }
    },
    async selectModel(modelName: string) {
      const settings = this.settings;
      if (!settings || !modelName) {
        return;
      }

      await this.updateSettings({
        model: modelName,
      });
    },
    async checkProviderStatus() {
      this.isCheckingProvider = true;
      this.error = null;

      try {
        this.providerStatus = await getAiProviderStatus();
        if (!this.isProviderReachable) {
          this.models = [];
        }
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isCheckingProvider = false;
      }
    },
    async startProvider() {
      this.isStartingProvider = true;
      this.error = null;

      try {
        this.providerStatus = await startLocalAiProvider();
        if (this.providerStatus.state === "starting") {
          await this.checkProviderStatus();
        }
        if (this.isProviderReachable) {
          await this.loadModels();
        }
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isStartingProvider = false;
      }
    },
    async loadModels() {
      if (!this.isProviderReachable) {
        this.models = [];
        return;
      }

      this.isLoadingModels = true;
      this.error = null;

      try {
        this.models = await listAiModels();
      } catch (error) {
        this.models = [];
        this.error = normalizeUiError(error);
      } finally {
        this.isLoadingModels = false;
      }
    },
    async sendChatMessage(
      content: string,
      sceneId?: string,
      includeSceneContext = true,
      mode: AiChatMode = "ideas",
    ) {
      const text = content.trim();
      if (!text) {
        return;
      }

      const userMessage: AiChatMessage = { role: "user", content: text };
      this.chatMessages = [...this.chatMessages, userMessage];
      this.isRunningChat = true;
      this.error = null;

      try {
        const response = await sendAiChatMessage({
          messages: this.chatMessages,
          sceneId,
          includeSceneContext: includeSceneContext && Boolean(sceneId),
          mode,
        });
        this.chatMessages = [...this.chatMessages, response.message];
      } catch (error) {
        this.chatMessages = this.chatMessages.slice(0, -1);
        this.error = normalizeUiError(error);
      } finally {
        this.isRunningChat = false;
      }
    },
    async loadSceneSuggestions(sceneId: string) {
      if (!sceneId) {
        this.reset();
        return;
      }

      this.isLoading = true;
      this.error = null;

      try {
        this.suggestions = await listAiSuggestionsForScene(sceneId);
        this.activeSuggestionId =
          this.suggestions.find((suggestion) => suggestion.status === "draft")?.id ??
          this.suggestions[0]?.id ??
          "";
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async loadBibleSuggestions(targetType?: ProjectBibleCardType, targetId?: string) {
      this.isLoadingBibleSuggestions = true;
      this.error = null;

      try {
        this.bibleSuggestions = await listAiBibleSuggestions(targetType, targetId);
        this.activeBibleSuggestionId =
          this.bibleSuggestions.find((suggestion) => suggestion.status === "draft")?.id ??
          this.bibleSuggestions[0]?.id ??
          "";
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoadingBibleSuggestions = false;
      }
    },
    async summarizeActiveScene(sceneId: string) {
      if (!sceneId) {
        return;
      }

      this.isRunning = true;
      this.error = null;

      try {
        const suggestion = await summarizeScene(sceneId);
        this.suggestions = [
          suggestion,
          ...this.suggestions.filter((item) => item.id !== suggestion.id),
        ];
        this.activeSuggestionId = suggestion.id;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isRunning = false;
      }
    },
    async checkActiveSceneContradictions(sceneId: string) {
      if (!sceneId) {
        return;
      }

      this.isRunning = true;
      this.error = null;

      try {
        const suggestion = await checkSceneContradictions(sceneId);
        this.suggestions = [
          suggestion,
          ...this.suggestions.filter((item) => item.id !== suggestion.id),
        ];
        this.activeSuggestionId = suggestion.id;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isRunning = false;
      }
    },
    async generateTextAlternatives(input: GenerateTextAlternativesInput) {
      this.isRunningAlternatives = true;
      this.error = null;
      this.textAlternatives = [];

      try {
        const response = await generateTextAlternatives(input);
        this.textAlternatives = response.alternatives;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isRunningAlternatives = false;
      }
    },
    async improveText(input: ImproveTextInput) {
      this.isRunningImprovements = true;
      this.error = null;
      this.textImprovements = [];

      try {
        const response = await improveText(input);
        this.textImprovements = response.improvements;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isRunningImprovements = false;
      }
    },
    async generateBibleCard(targetType: ProjectBibleCardType, keywords: string) {
      this.isRunningBible = true;
      this.error = null;

      try {
        const suggestion = await generateBibleCardFromKeywords(targetType, keywords);
        this.bibleSuggestions = [
          suggestion,
          ...this.bibleSuggestions.filter((item) => item.id !== suggestion.id),
        ];
        this.activeBibleSuggestionId = suggestion.id;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isRunningBible = false;
      }
    },
    async completeBibleCard(targetType: ProjectBibleCardType, targetId: string, keywords: string) {
      if (!targetId) {
        return;
      }

      this.isRunningBible = true;
      this.error = null;

      try {
        const suggestion = await completeBibleCardFromKeywords(targetType, targetId, keywords);
        this.bibleSuggestions = [
          suggestion,
          ...this.bibleSuggestions.filter((item) => item.id !== suggestion.id),
        ];
        this.activeBibleSuggestionId = suggestion.id;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isRunningBible = false;
      }
    },
    async editBibleCard(targetType: ProjectBibleCardType, targetId: string, instruction: string) {
      if (!targetId) return;
      this.isRunningBible = true;
      this.error = null;
      try {
        const suggestion = await editBibleCard(targetType, targetId, instruction);
        this.bibleSuggestions = [suggestion, ...this.bibleSuggestions.filter((item) => item.id !== suggestion.id)];
        this.activeBibleSuggestionId = suggestion.id;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isRunningBible = false;
      }
    },
    async acceptActiveSuggestion() {
      const suggestion = this.activeSuggestion;
      if (!suggestion) {
        return null;
      }

      this.isSaving = true;
      this.error = null;

      try {
        const accepted = await acceptAiSuggestion(suggestion.id);
        this.suggestions = this.suggestions.map((item) =>
          item.id === accepted.id ? accepted : item,
        );
        this.activeSuggestionId = accepted.id;
        return accepted;
      } catch (error) {
        this.error = normalizeUiError(error);
        return null;
      } finally {
        this.isSaving = false;
      }
    },
    async acceptActiveBibleSuggestion(draft: AiBibleCardDraft, selectedFields: AiBibleEditableField[] = []): Promise<AiBibleAcceptResult | null> {
      const suggestion = this.activeBibleSuggestion;
      if (!suggestion) {
        return null;
      }

      this.isSaving = true;
      this.error = null;

      try {
        const result = await acceptAiBibleSuggestion(suggestion.id, draft, selectedFields);
        this.bibleSuggestions = this.bibleSuggestions.map((item) =>
          item.id === suggestion.id ? { ...item, status: "accepted" } : item,
        );
        this.activeBibleSuggestionId = suggestion.id;
        return result;
      } catch (error) {
        this.error = normalizeUiError(error);
        return null;
      } finally {
        this.isSaving = false;
      }
    },
    async rejectActiveSuggestion() {
      const suggestion = this.activeSuggestion;
      if (!suggestion) {
        return;
      }

      this.isSaving = true;
      this.error = null;

      try {
        const rejected = await rejectAiSuggestion(suggestion.id);
        this.suggestions = this.suggestions.map((item) =>
          item.id === rejected.id ? rejected : item,
        );
        this.activeSuggestionId = rejected.id;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isSaving = false;
      }
    },
    async rejectActiveBibleSuggestion() {
      const suggestion = this.activeBibleSuggestion;
      if (!suggestion) {
        return;
      }

      this.isSaving = true;
      this.error = null;

      try {
        const rejected = await rejectAiBibleSuggestion(suggestion.id);
        this.bibleSuggestions = this.bibleSuggestions.map((item) =>
          item.id === rejected.id ? rejected : item,
        );
        this.activeBibleSuggestionId = rejected.id;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isSaving = false;
      }
    },
  },
});
