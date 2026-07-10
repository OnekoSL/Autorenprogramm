import { invoke } from "@tauri-apps/api/core";
import type { NoteType, ProjectBibleCardType, ProjectBibleSnapshot } from "./projectBible";
import { requireTauriRuntime } from "./tauri";

export type AiActionType = "summarize_scene" | "check_scene_contradictions";
export type AiSuggestionStatus = "draft" | "accepted" | "rejected";
export type AiProviderState = "reachable" | "starting" | "unreachable" | "unsupported" | "error";

export interface AiSettings {
  model: string;
}

export interface UpdateAiSettingsInput {
  model: string;
}

export interface AiChatMessage {
  role: "user" | "assistant";
  content: string;
}

export type AiChatMode = "ideas" | "writing";

export interface SendAiChatMessageInput {
  messages: AiChatMessage[];
  sceneId?: string;
  includeSceneContext: boolean;
  mode: AiChatMode;
}

export interface AiChatResponse {
  message: AiChatMessage;
}

export interface GenerateTextAlternativesInput {
  sceneId: string;
  selectedText: string;
  contextBefore: string;
  contextAfter: string;
}

export interface TextAlternativesResponse {
  alternatives: string[];
}

export type ImproveTextMode = "selection" | "scene";

export interface ImproveTextInput {
  sceneId: string;
  selectedText: string;
  contextBefore: string;
  contextAfter: string;
  mode: ImproveTextMode;
}

export interface TextImprovementsResponse {
  improvements: string[];
}

export interface AiProviderStatus {
  state: AiProviderState;
  message: string;
  canStart: boolean;
  baseUrl: string;
  model: string;
}

export interface AiModelInfo {
  id: string;
  name: string;
  size: number;
  modifiedAt: string;
}

export interface AiSuggestionSnapshot {
  id: string;
  projectId: string;
  sceneId: string;
  actionType: AiActionType;
  inputExcerpt: string;
  contextSummary: string;
  result: string;
  status: AiSuggestionStatus;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export type AiBibleSuggestionMode = "generate" | "complete";

export interface AiBibleCardDraft {
  name: string;
  role: string;
  description: string;
  notes: string;
  title: string;
  body: string;
  noteType: NoteType;
}

export interface AiBibleSuggestionSnapshot {
  id: string;
  projectId: string;
  targetType: ProjectBibleCardType;
  targetId?: string | null;
  mode: AiBibleSuggestionMode;
  keywords: string;
  draft: AiBibleCardDraft;
  status: AiSuggestionStatus;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export interface AiBibleAcceptResult {
  snapshot: ProjectBibleSnapshot;
  targetType: ProjectBibleCardType;
  targetId: string;
}

export function getAiSettings(): Promise<AiSettings> {
  requireTauriRuntime();
  return invoke<AiSettings>("get_ai_settings");
}

export function updateAiSettings(input: UpdateAiSettingsInput): Promise<AiSettings> {
  requireTauriRuntime();
  return invoke<AiSettings>("update_ai_settings", { input });
}

export function getAiProviderStatus(): Promise<AiProviderStatus> {
  requireTauriRuntime();
  return invoke<AiProviderStatus>("get_ai_provider_status");
}

export function startLocalAiProvider(): Promise<AiProviderStatus> {
  requireTauriRuntime();
  return invoke<AiProviderStatus>("start_local_ai_provider");
}

export function listAiModels(): Promise<AiModelInfo[]> {
  requireTauriRuntime();
  return invoke<AiModelInfo[]>("list_ai_models");
}

export function sendAiChatMessage(input: SendAiChatMessageInput): Promise<AiChatResponse> {
  requireTauriRuntime();
  return invoke<AiChatResponse>("send_ai_chat_message", { input });
}

export function generateTextAlternatives(
  input: GenerateTextAlternativesInput,
): Promise<TextAlternativesResponse> {
  requireTauriRuntime();
  return invoke<TextAlternativesResponse>("generate_text_alternatives", { input });
}

export function improveText(input: ImproveTextInput): Promise<TextImprovementsResponse> {
  requireTauriRuntime();
  return invoke<TextImprovementsResponse>("improve_text", { input });
}

export function summarizeScene(sceneId: string): Promise<AiSuggestionSnapshot> {
  requireTauriRuntime();
  return invoke<AiSuggestionSnapshot>("summarize_scene", { input: { sceneId } });
}

export function checkSceneContradictions(sceneId: string): Promise<AiSuggestionSnapshot> {
  requireTauriRuntime();
  return invoke<AiSuggestionSnapshot>("check_scene_contradictions", { input: { sceneId } });
}

export function listAiSuggestionsForScene(sceneId: string): Promise<AiSuggestionSnapshot[]> {
  requireTauriRuntime();
  return invoke<AiSuggestionSnapshot[]>("list_ai_suggestions_for_scene", { input: { sceneId } });
}

export function acceptAiSuggestion(id: string): Promise<AiSuggestionSnapshot> {
  requireTauriRuntime();
  return invoke<AiSuggestionSnapshot>("accept_ai_suggestion", { input: { id } });
}

export function rejectAiSuggestion(id: string): Promise<AiSuggestionSnapshot> {
  requireTauriRuntime();
  return invoke<AiSuggestionSnapshot>("reject_ai_suggestion", { input: { id } });
}

export function generateBibleCardFromKeywords(
  targetType: ProjectBibleCardType,
  keywords: string,
): Promise<AiBibleSuggestionSnapshot> {
  requireTauriRuntime();
  return invoke<AiBibleSuggestionSnapshot>("generate_bible_card_from_keywords", {
    input: { targetType, keywords },
  });
}

export function completeBibleCardFromKeywords(
  targetType: ProjectBibleCardType,
  targetId: string,
  keywords: string,
): Promise<AiBibleSuggestionSnapshot> {
  requireTauriRuntime();
  return invoke<AiBibleSuggestionSnapshot>("complete_bible_card_from_keywords", {
    input: { targetType, targetId, keywords },
  });
}

export function listAiBibleSuggestions(
  targetType?: ProjectBibleCardType,
  targetId?: string,
): Promise<AiBibleSuggestionSnapshot[]> {
  requireTauriRuntime();
  return invoke<AiBibleSuggestionSnapshot[]>("list_ai_bible_suggestions", {
    input: { targetType, targetId },
  });
}

export function acceptAiBibleSuggestion(
  id: string,
  draft: AiBibleCardDraft,
): Promise<AiBibleAcceptResult> {
  requireTauriRuntime();
  return invoke<AiBibleAcceptResult>("accept_ai_bible_suggestion", { input: { id, draft } });
}

export function rejectAiBibleSuggestion(id: string): Promise<AiBibleSuggestionSnapshot> {
  requireTauriRuntime();
  return invoke<AiBibleSuggestionSnapshot>("reject_ai_bible_suggestion", { input: { id } });
}
