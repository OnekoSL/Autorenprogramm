import { invoke } from "@tauri-apps/api/core";
import { requireTauriRuntime } from "./tauri";
export type ProjectBibleCardType = "character" | "location" | "item" | "note" | "faction" | "event";
export type KnowledgeTargetType = ProjectBibleCardType | "world_system";
export type NoteType = "general" | "research" | "lore" | "faction" | "event" | "todo";

export interface ProjectBibleSnapshot {
  characters: CharacterSnapshot[];
  locations: LocationSnapshot[];
  items: ItemSnapshot[];
  notes: NoteSnapshot[];
  factions: FactionSnapshot[];
  events: EventSnapshot[];
  sceneLinks: SceneLinkSnapshot[];
  relationships: BibleRelationshipSnapshot[];
}

export interface CharacterSnapshot {
  id: string;
  projectId: string;
  name: string;
  role: string;
  description: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export interface LocationSnapshot {
  id: string;
  projectId: string;
  name: string;
  description: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export interface ItemSnapshot {
  id: string;
  projectId: string;
  name: string;
  description: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export interface NoteSnapshot {
  id: string;
  projectId: string;
  title: string;
  body: string;
  noteType: NoteType;
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export interface FactionSnapshot { id: string; projectId: string; name: string; factionType: string; description: string; goals: string; notes: string; createdAt: string; updatedAt: string; deletedAt?: string | null; }
export interface EventSnapshot { id: string; projectId: string; title: string; timeLabel: string; summary: string; outcome: string; notes: string; sortOrder: number; createdAt: string; updatedAt: string; deletedAt?: string | null; }
export type BibleRelationshipType = "related" | "knows" | "family" | "allied" | "enemy" | "member_of" | "leads" | "owns" | "located_in" | "created_by" | "participates_in" | "caused" | "follows" | "uses_system" | "depends_on" | "affects" | "created_by_system";
export interface BibleRelationshipSnapshot { id: string; sourceType: KnowledgeTargetType; sourceId: string; targetType: KnowledgeTargetType; targetId: string; relationshipType: BibleRelationshipType; description: string; createdAt: string; updatedAt: string; }
export interface BibleRelationshipUpdate { id?: string; sourceType: KnowledgeTargetType; sourceId: string; targetType: KnowledgeTargetType; targetId: string; relationshipType: BibleRelationshipType; description: string; }

export interface SceneLinkSnapshot {
  id: string;
  sceneId: string;
  targetType: KnowledgeTargetType;
  targetId: string;
  relation: "assigned";
  sortOrder: number;
  sceneTitle: string;
  chapterTitle: string;
  areaTitle: string;
}

export interface ProjectBibleCardUpdate {
  targetType: ProjectBibleCardType;
  id: string;
  name?: string;
  role?: string;
  description?: string;
  notes?: string;
  title?: string;
  body?: string;
  noteType?: NoteType;
}

export function getProjectBible(): Promise<ProjectBibleSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectBibleSnapshot>("get_project_bible");
}

export function createProjectBibleCard(
  targetType: ProjectBibleCardType,
): Promise<ProjectBibleSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectBibleSnapshot>("create_project_bible_card", {
    input: { targetType },
  });
}

export function updateProjectBibleCard(
  input: ProjectBibleCardUpdate,
): Promise<ProjectBibleSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectBibleSnapshot>("update_project_bible_card", { input });
}

export function deleteProjectBibleCard(
  targetType: ProjectBibleCardType,
  id: string,
): Promise<ProjectBibleSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectBibleSnapshot>("delete_project_bible_card", {
    input: { targetType, id },
  });
}

export function getSceneLinksForCard(
  targetType: KnowledgeTargetType,
  targetId: string,
): Promise<SceneLinkSnapshot[]> {
  requireTauriRuntime();
  return invoke<SceneLinkSnapshot[]>("get_scene_links_for_card", {
    input: { targetType, targetId },
  });
}

export function replaceSceneLinksForCard(
  targetType: KnowledgeTargetType,
  targetId: string,
  sceneIds: string[],
): Promise<ProjectBibleSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectBibleSnapshot>("replace_scene_links_for_card", {
    input: { targetType, targetId, sceneIds },
  });
}

export function upsertBibleRelationship(input: BibleRelationshipUpdate): Promise<ProjectBibleSnapshot> { requireTauriRuntime(); return invoke<ProjectBibleSnapshot>("upsert_bible_relationship", { input }); }
export function deleteBibleRelationship(id: string): Promise<ProjectBibleSnapshot> { requireTauriRuntime(); return invoke<ProjectBibleSnapshot>("delete_bible_relationship", { input: { id } }); }
export function reorderBibleEvents(eventIds: string[]): Promise<ProjectBibleSnapshot> { requireTauriRuntime(); return invoke<ProjectBibleSnapshot>("reorder_bible_events", { input: { eventIds } }); }
