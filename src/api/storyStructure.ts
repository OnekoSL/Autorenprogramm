import { invoke } from "@tauri-apps/api/core";
import { requireTauriRuntime } from "./tauri";

export type StructureCategory = "plot" | "genre" | "character_arc" | "planning_method" | "custom";
export type StructureBeatStatus = "empty" | "planned" | "in_progress" | "complete";

export interface StructureBeat {
  id: string;
  title: string;
  description: string;
  summary: string;
  notes: string;
  guidingQuestions: string[];
  required: boolean;
  suggestedPercentageStart?: number;
  suggestedPercentageEnd?: number;
  order: number;
  status?: StructureBeatStatus;
  manuscriptChapterId?: string;
  sceneIds: string[];
  characterIds: string[];
}

export interface StructureAct {
  id: string;
  title: string;
  description: string;
  order: number;
  beats: StructureBeat[];
  manuscriptAreaId?: string;
}

export interface StoryStructureTemplate {
  formatVersion: number;
  id: string;
  name: string;
  description: string;
  category: StructureCategory;
  tags: string[];
  planningMethods: PlanningMethodInfo[];
  acts: StructureAct[];
}

export interface PlanningMethodInfo {
  id: string;
  name: string;
  description: string;
  steps: string[];
}

export interface ProjectStructure {
  id: string;
  novelId: string;
  templateId: string;
  name: string;
  description: string;
  category: StructureCategory;
  acts: StructureAct[];
  createdAt: string;
  updatedAt: string;
}

export function listStoryStructureTemplates(): Promise<StoryStructureTemplate[]> {
  requireTauriRuntime();
  return invoke("list_story_structure_templates");
}

export function getProjectStructure(): Promise<ProjectStructure | null> {
  requireTauriRuntime();
  return invoke("get_project_structure");
}

export function applyStoryStructureTemplate(
  templateId: string,
  replaceExisting: boolean,
): Promise<ProjectStructure> {
  requireTauriRuntime();
  return invoke("apply_story_structure_template", { input: { templateId, replaceExisting } });
}

export function generateStoryStructureManuscript(): Promise<ProjectStructure> {
  requireTauriRuntime();
  return invoke("generate_story_structure_manuscript");
}

export interface StructureBeatUpdate { id: string; title: string; description: string; summary: string; notes: string; status: StructureBeatStatus; }
export function updateStructureBeat(input: StructureBeatUpdate): Promise<ProjectStructure> { requireTauriRuntime(); return invoke("update_structure_beat", { input }); }
export function replaceStructureBeatSceneLinks(beatId: string, targetIds: string[]): Promise<ProjectStructure> { requireTauriRuntime(); return invoke("replace_structure_beat_scene_links", { input: { beatId, targetIds } }); }
export function replaceStructureBeatCharacterLinks(beatId: string, targetIds: string[]): Promise<ProjectStructure> { requireTauriRuntime(); return invoke("replace_structure_beat_character_links", { input: { beatId, targetIds } }); }
