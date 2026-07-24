import { invoke } from "@tauri-apps/api/core";
import { requireTauriRuntime } from "./tauri";
export type SceneStatus = "draft" | "revision" | "complete";

export interface ManuscriptTree {
  novel: NovelSnapshot;
  areas: AreaSnapshot[];
}

export interface NovelSnapshot {
  id: string;
  title: string;
  subtitle: string;
  authorName: string;
}

export interface AreaSnapshot {
  id: string;
  title: string;
  summary: string;
  sortOrder: number;
  chapters: ChapterSnapshot[];
}

export interface ChapterSnapshot {
  id: string;
  areaId: string;
  title: string;
  summary: string;
  sortOrder: number;
  scenes: SceneSnapshot[];
}

export interface SceneSnapshot {
  id: string;
  chapterId: string;
  title: string;
  text: string;
  status: SceneStatus;
  summary: string;
  sortOrder: number;
}

export interface CreateChapterInput {
  areaId?: string;
  title?: string;
  afterChapterId?: string;
}

export interface CreateSceneInput {
  chapterId?: string;
  title?: string;
  afterSceneId?: string;
}

export interface UpdateChapterInput {
  id: string;
  title: string;
  summary: string;
}

export interface CreateAreaInput {
  title?: string;
  afterAreaId?: string;
}

export interface UpdateAreaInput {
  id: string;
  title: string;
  summary: string;
}

export interface UpdateSceneInput {
  id: string;
  title: string;
  text: string;
  status: SceneStatus;
  summary: string;
}

export interface MoveChapterInput {
  chapterId: string;
  targetAreaId: string;
  targetIndex: number;
}

export interface MoveAreaInput {
  areaId: string;
  targetIndex: number;
}

export interface MoveSceneInput {
  sceneId: string;
  targetChapterId: string;
  targetIndex: number;
}

export interface DeleteChapterInput {
  id: string;
}

export interface DeleteAreaInput {
  id: string;
}

export interface DeleteSceneInput {
  id: string;
}

export function getManuscriptTree(): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("get_manuscript_tree");
}

export function createArea(input: CreateAreaInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("create_area", { input });
}

export function createChapter(input: CreateChapterInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("create_chapter", { input });
}

export function createScene(input: CreateSceneInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("create_scene", { input });
}

export function updateArea(input: UpdateAreaInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("update_area", { input });
}

export function updateChapter(input: UpdateChapterInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("update_chapter", { input });
}

export function updateScene(input: UpdateSceneInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("update_scene", { input });
}

export function moveArea(input: MoveAreaInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("move_area", { input });
}

export function moveChapter(input: MoveChapterInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("move_chapter", { input });
}

export function moveScene(input: MoveSceneInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("move_scene", { input });
}

export function deleteArea(input: DeleteAreaInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("delete_area", { input });
}

export function deleteChapter(input: DeleteChapterInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("delete_chapter", { input });
}

export function deleteScene(input: DeleteSceneInput): Promise<ManuscriptTree> {
  requireTauriRuntime();
  return invoke<ManuscriptTree>("delete_scene", { input });
}
