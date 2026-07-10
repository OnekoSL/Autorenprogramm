import { invoke } from "@tauri-apps/api/core";
import { requireTauriRuntime } from "./tauri";

export type ProjectBibleCardType = "character" | "location" | "item" | "note";
export type NoteType = "general" | "research" | "lore" | "faction" | "event" | "todo";

export interface ProjectBibleSnapshot {
  characters: CharacterSnapshot[];
  locations: LocationSnapshot[];
  items: ItemSnapshot[];
  notes: NoteSnapshot[];
  sceneLinks: SceneLinkSnapshot[];
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

export interface SceneLinkSnapshot {
  id: string;
  sceneId: string;
  targetType: ProjectBibleCardType;
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
  targetType: ProjectBibleCardType,
  targetId: string,
): Promise<SceneLinkSnapshot[]> {
  requireTauriRuntime();
  return invoke<SceneLinkSnapshot[]>("get_scene_links_for_card", {
    input: { targetType, targetId },
  });
}

export function replaceSceneLinksForCard(
  targetType: ProjectBibleCardType,
  targetId: string,
  sceneIds: string[],
): Promise<ProjectBibleSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectBibleSnapshot>("replace_scene_links_for_card", {
    input: { targetType, targetId, sceneIds },
  });
}
