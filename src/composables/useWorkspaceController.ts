
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AiBibleCardDraft, AiBibleEditableField, AiChatMode, ImproveTextMode } from "../api/ai";
import { chooseExportPath, type ExportFormat, type ExportProfile } from "../api/export";
import type { AreaSnapshot, ChapterSnapshot, SceneSnapshot, SceneStatus, UpdateSceneInput } from "../api/manuscript";
import { chooseProjectBackupPath } from "../api/project";
import { localeCode, projectLanguage, setLocale, type ProjectLanguage } from "../i18n";
import { formatUiError } from "../errors";
import generatedUi from "../locales/generated-ui.json";
import type { BibleRelationshipType, KnowledgeTargetType, NoteType, ProjectBibleCardType, ProjectBibleCardUpdate } from "../api/projectBible";
import type { SearchProjectResult } from "../api/search";
import type { StructureBeatStatus, StructureBeatUpdate } from "../api/storyStructure";
import type { WorldSystemCategory, WorldSystemUpdate } from "../api/worldbuilding";
import { createDraftAutosave } from "./draftAutosave";
import { isRelationshipTypeAllowed, relationshipDefinitions, relationshipDisplayLabel } from "../domain/relationships";
import { useAiStore } from "../stores/ai";
import { useExportStore } from "../stores/export";
import { useManuscriptStore } from "../stores/manuscript";
import { useProjectStore } from "../stores/project";
import { useSearchStore } from "../stores/search";
import { useStoryStructureStore } from "../stores/storyStructure";
import { useWorldbuildingStore, worldSystemCategories as worldSystemCategoryDefinitions } from "../stores/worldbuilding";
import {
  noteTypes as noteTypeDefinitions,
  projectBibleCardTypes as projectBibleCardTypeDefinitions,
  useProjectBibleStore,
  type KnowledgeCard,
  type ProjectBibleCard,
} from "../stores/projectBible";
import {
  defaultWorkspaceLayout,
  useWorkspaceStore,
  type LayoutPanel,
  type SidebarTab,
  type WorkspaceLayout,
  type WorkspaceSection,
} from "../stores/workspace";
import type { WorkspaceNavigation } from "./workspaceNavigation";

export function shouldExcludeRelationshipCandidate(
  candidate: Pick<KnowledgeCard, "targetType" | "id">,
  activeSection: WorkspaceSection,
  selectedCard: Pick<KnowledgeCard, "targetType" | "id"> | null,
  selectedSystemId: string,
): boolean {
  if (activeSection === "worldbuilding") {
    return candidate.targetType === "world_system" && candidate.id === selectedSystemId;
  }

  return activeSection === "bible"
    && candidate.targetType === selectedCard?.targetType
    && candidate.id === selectedCard.id;
}

export function useWorkspaceController(workspaceNavigation: WorkspaceNavigation) {

type DragPayload =
  | { type: "area"; areaId: string }
  | { type: "chapter"; chapterId: string; areaId: string }
  | { type: "scene"; sceneId: string; chapterId: string };

type DropTarget =
  | { kind: "area"; areaId: string; areaIndex: number }
  | { kind: "areaEnd"; areaId: string }
  | { kind: "chapter"; areaId: string; chapterId: string; chapterIndex: number }
  | { kind: "scene"; chapterId: string; sceneIndex: number }
  | { kind: "chapterEnd"; chapterId: string };

interface PointerDragState {
  payload: DragPayload;
  startX: number;
  startY: number;
  active: boolean;
}

interface SceneTextHistory {
  undo: string[];
  redo: string[];
  current: string;
}

const workspace = useWorkspaceStore();
const { locale, t } = useI18n();
function ui(source: string) { return locale.value === "en" ? (generatedUi as Record<string, string>)[source] ?? source : source; }
const projectBibleCardTypes = computed(() => projectBibleCardTypeDefinitions.map((item) => ({ ...item, label: t(item.labelKey) })));
const noteTypes = computed(() => noteTypeDefinitions.map((item) => ({ ...item, label: t(item.labelKey) })));
const worldSystemCategories = computed(() => worldSystemCategoryDefinitions.map((item) => ({ ...item, label: t(item.labelKey) })));
const projectStore = useProjectStore();
const manuscriptStore = useManuscriptStore();
const projectBibleStore = useProjectBibleStore();
const aiStore = useAiStore();
const searchStore = useSearchStore();
const exportStore = useExportStore();
const storyStructureStore = useStoryStructureStore();
const worldbuildingStore = useWorldbuildingStore();
const pointerDrag = ref<PointerDragState | null>(null);
const pointerDropKey = ref("");
const focusedNavigatorKey = ref("");
const collapsedAreaIds = ref(new Set<string>());
const collapsedChapterIds = ref(new Set<string>());
const isHydratingSceneDraft = ref(false);
const isHydratingCardDraft = ref(false);
const isHydratingWorldSystemDraft = ref(false);
const isApplyingTextHistory = ref(false);
const layoutStorageKey = "autorenprogramm.workspace.layout";
const navigatorCollapseStorageKeyPrefix = "autorenprogramm.navigator.collapse";
const sceneTextHistoryLimit = 50;
let searchTimer: number | undefined;
const isHydratingBeatDraft = ref(false);
const writingBibleOpenCardKey = ref("");
const writingBibleAddType = ref<ProjectBibleCardType>("character");
const writingBibleSearchQuery = ref("");
const relationshipTargetType = ref<KnowledgeTargetType>("character");
const relationshipTargetId = ref("");
const relationshipType = ref<BibleRelationshipType>("related");
const relationshipDescription = ref("");
const relationshipEditId = ref("");
const relationshipSearch = ref("");
const draggedEventId = ref("");
const showWorldSystemDialog = ref(false);
const newWorldSystemName = ref("");
const newWorldSystemCategory = ref<WorldSystemCategory>("magic");
const newWorldSystemTemplateId = ref("magic");
const worldSystemDraft = reactive<WorldSystemUpdate>({ id: "", name: "", category: "magic", templateId: "magic", principle: "", rules: "", capabilities: "", costs: "", limits: "", access: "", distribution: "", worldImpact: "", openQuestions: "", notes: "" });
const beatDraft = reactive({ id: "", title: "", description: "", summary: "", notes: "", status: "empty" as StructureBeatStatus });
const worldSystemFieldLabels = computed(() => ({
  name: ui("Name"), principle: ui("Grundprinzip"), rules: ui("Regeln"), capabilities: ui("Möglichkeiten"),
  costs: ui("Kosten"), limits: ui("Grenzen"), access: ui("Zugang"), distribution: ui("Verbreitung"),
  worldImpact: ui("Auswirkungen auf die Welt"), openQuestions: ui("Offene Fragen"), notes: ui("Notizen"),
}));
const worldSystemOptionalFields = computed<Array<{ key: keyof WorldSystemUpdate; label: string }>>(() => [
  { key: "principle", label: worldSystemFieldLabels.value.principle }, { key: "rules", label: worldSystemFieldLabels.value.rules },
  { key: "capabilities", label: worldSystemFieldLabels.value.capabilities }, { key: "costs", label: worldSystemFieldLabels.value.costs },
  { key: "limits", label: worldSystemFieldLabels.value.limits }, { key: "access", label: worldSystemFieldLabels.value.access },
  { key: "distribution", label: worldSystemFieldLabels.value.distribution }, { key: "worldImpact", label: worldSystemFieldLabels.value.worldImpact },
  { key: "openQuestions", label: worldSystemFieldLabels.value.openQuestions }, { key: "notes", label: worldSystemFieldLabels.value.notes },
]);

const sceneStatuses = computed(() => [
  { value: "draft" as SceneStatus, label: t("writing.draft") },
  { value: "revision" as SceneStatus, label: t("writing.revision") },
  { value: "complete" as SceneStatus, label: t("writing.complete") },
]);

function sceneStatusLabel(value: string) {
  return sceneStatuses.value.find((status) => status.value === value)?.label ?? value;
}
type EditorFontFamily = "serif" | "sans" | "monospace";
type BibleAiMode = "generate" | "edit";
type SidebarAiAction = "summary" | "contradictions" | "alternatives" | "improve";
const editorFontOptions: Array<{ id: EditorFontFamily; label: string }> = [
  { id: "serif", label: "Serif" },
  { id: "sans", label: "Sans" },
  { id: "monospace", label: "Monospace" },
];
const editorFontSizeOptions = [16, 17, 18, 19, 20, 21, 22, 24];
const editorFontStacks: Record<EditorFontFamily, string> = {
  serif: 'Georgia, "Times New Roman", serif',
  sans: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  monospace: '"Cascadia Mono", Consolas, "Courier New", monospace',
};
const sceneDraft = reactive<{ id: string; title: string; status: SceneStatus; summary: string; text: string }>({
  id: "",
  title: "",
  status: "draft",
  summary: "",
  text: "",
});
const cardDraft = reactive<{
  id: string;
  targetType: ProjectBibleCardType;
  name: string;
  role: string;
  description: string;
  notes: string;
  title: string;
  body: string;
  noteType: NoteType;
}>({
  id: "",
  targetType: "character",
  name: "",
  role: "",
  description: "",
  notes: "",
  title: "",
  body: "",
  noteType: "general",
});
const sceneAutosave = createDraftAutosave<UpdateSceneInput>({
  delay: 400,
  save: (input) => manuscriptStore.updateScene(input),
});
const cardAutosave = createDraftAutosave<ProjectBibleCardUpdate>({
  delay: 400,
  save: (input) => projectBibleStore.updateCard(input),
});
const worldSystemAutosave = createDraftAutosave<WorldSystemUpdate>({
  delay: 450,
  save: (input) => worldbuildingStore.update(input),
});
const beatAutosave = createDraftAutosave<StructureBeatUpdate>({
  delay: 400,
  save: (input) => storyStructureStore.updateBeat(input),
});
const worldSystemSavePending = worldSystemAutosave.pending;
const projectSettingsDraft = reactive({
  name: "",
  description: "",
  defaultLanguage: "de" as ProjectLanguage,
  editorFontFamily: "serif" as EditorFontFamily,
  editorFontSize: 19,
});
const projectDefaultsTemplateId = ref("freestyle");
const aiChatDraft = ref("");
const aiChatIncludeSceneContext = ref(true);
const aiChatMode = ref<AiChatMode>("ideas");
const sidebarAiAction = ref<SidebarAiAction>("summary");
const sceneTextAreaRef = ref<HTMLTextAreaElement | null>(null);
const sceneTextSelection = reactive({
  start: 0,
  end: 0,
  text: "",
  sceneId: "",
});
const sceneTextHistories = reactive<Record<string, SceneTextHistory>>({});
const textImprovementTarget = reactive({
  mode: "" as ImproveTextMode | "",
  start: 0,
  end: 0,
  text: "",
  sceneId: "",
});
const bibleAiMode = ref<BibleAiMode>("generate");
const bibleAiKeywords = ref("");
const bibleAiSelectedFields = ref<AiBibleEditableField[]>([]);
const bibleAiDraft = reactive<AiBibleCardDraft>({
  name: "",
  role: "",
  description: "",
  notes: "",
  title: "",
  body: "",
  noteType: "general",
});

const panelBounds: Record<LayoutPanel, { min: number; max: number }> = {
  navigator: { min: 220, max: 460 },
  sidebar: { min: 280, max: 560 },
};

const sidebarTabs = computed<Array<{ id: SidebarTab; label: string }>>(() => {
  if (workspace.activeSection === "structure") return [{ id: "context", label: ui("Struktur") }, { id: "world", label: ui("Welt") }];
  if (workspace.activeSection === "bible") {
    return [
      { id: "context", label: ui("Szenen") }, { id: "bible", label: ui("Bibel") }, { id: "world", label: ui("Welt") }, { id: "ai", label: ui("KI") },
    ];
  }

  return [
    { id: "context", label: ui("Kontext") }, { id: "bible", label: ui("Bibel") }, { id: "world", label: ui("Welt") }, { id: "ai", label: ui("KI") },
  ];
});

const sidebarTitle = computed(() => {
  if (workspace.activeSection === "structure") {
    return workspace.sidebarTab === "world" ? "Worldbuilding" : ui("Strukturdetails");
  }
  if (workspace.activeSection === "bible") {
    if (workspace.sidebarTab === "context") {
      return ui("Szenenzuordnung");
    }

    if (workspace.sidebarTab === "ai") {
      return ui("Karten-KI");
    }

    return ui("Kartenübersicht");
  }

  if (workspace.sidebarTab === "ai") {
    return ui("KI-Seitenleiste");
  }

  if (workspace.sidebarTab === "bible") {
    return ui("Projektbibel");
  }
  if (workspace.sidebarTab === "world") return "Worldbuilding";

  return ui("Szenenkontext");
});

const structureCategoryLabels = computed<Record<string, string>>(() => ({ plot: ui("Plotstruktur"), genre: ui("Genrevorlage"), character_arc: ui("Figurenbogen"), planning_method: ui("Planungsmethode"), custom: ui("Eigene Struktur") }));

const structureBeatCount = computed(() =>
  storyStructureStore.displayedActs.reduce((total, act) => total + act.beats.length, 0),
);

const activeSceneStructureAssignments = computed(() => {
  const structure = storyStructureStore.activeStructure;
  const sceneId = manuscriptStore.activeSceneId;
  if (!structure || !sceneId) return [];
  return structure.acts.flatMap((act) => act.beats.filter((beat) => beat.sceneIds.includes(sceneId)).map((beat) => ({ act, beat })));
});
const structureCharacters = computed(() => [...(projectBibleStore.snapshot?.characters ?? [])].sort((a, b) => a.name.localeCompare(b.name, locale.value === "en" ? "en-GB" : "de-DE")));

function structureAreaTitle(areaId?: string) {
  return manuscriptStore.areas.find((area) => area.id === areaId)?.title ?? "";
}

function structureChapterTitle(chapterId?: string) {
  return manuscriptStore.chapters.find((chapter) => chapter.id === chapterId)?.title ?? "";
}

function hydrateBeatDraft() {
  isHydratingBeatDraft.value = true;
  beatAutosave.reset();
  const beat = storyStructureStore.selectedBeat;
  Object.assign(beatDraft, beat ? { id: beat.id, title: beat.title, description: beat.description, summary: beat.summary || "", notes: beat.notes || "", status: (beat.status || "empty") as StructureBeatStatus } : { id: "", title: "", description: "", summary: "", notes: "", status: "empty" });
  window.setTimeout(() => { isHydratingBeatDraft.value = false; });
}

function scheduleBeatSave() {
  if (isHydratingBeatDraft.value || !beatDraft.id || structureIsPreview.value) return;
  beatAutosave.schedule({ ...beatDraft });
}

async function flushBeatSave(): Promise<boolean> {
  return beatAutosave.flush();
}

async function selectStructureBeat(id: string) {
  if (id === storyStructureStore.selectedBeatId) return;
  if (!await flushBeatSave()) return;
  storyStructureStore.selectBeat(id);
}

async function toggleBeatScene(sceneId: string, linked: boolean) {
  const beat = storyStructureStore.selectedBeat;
  if (!beat || structureIsPreview.value) return;
  const ids = new Set(beat.sceneIds);
  linked ? ids.add(sceneId) : ids.delete(sceneId);
  await storyStructureStore.replaceBeatSceneLinks(beat.id, [...ids]);
}

async function toggleBeatCharacter(characterId: string, linked: boolean) {
  const beat = storyStructureStore.selectedBeat;
  if (!beat || structureIsPreview.value) return;
  const ids = new Set(beat.characterIds);
  linked ? ids.add(characterId) : ids.delete(characterId);
  await storyStructureStore.replaceBeatCharacterLinks(beat.id, [...ids]);
}

async function toggleActiveSceneBeat(beatId: string, linked: boolean) {
  const beat = storyStructureStore.activeStructure?.acts.flatMap((act) => act.beats).find((item) => item.id === beatId);
  const sceneId = manuscriptStore.activeSceneId;
  if (!beat || !sceneId) return;
  const ids = new Set(beat.sceneIds);
  linked ? ids.add(sceneId) : ids.delete(sceneId);
  await storyStructureStore.replaceBeatSceneLinks(beat.id, [...ids]);
}

const structureIsPreview = computed(
  () => storyStructureStore.activeStructure?.templateId !== storyStructureStore.selectedTemplateId,
);

function structurePositionLabel(start?: number, end?: number) {
  if (start == null && end == null) return t("structure.noPosition");
  if (start === end || end == null) return t("structure.aboutPosition", { value: start });
  return `${start ?? 0}-${end} %`;
}

async function applySelectedStructureTemplate() {
  if (!storyStructureStore.selectedTemplateId || !structureIsPreview.value) return;
  const replacing = Boolean(storyStructureStore.activeStructure);
  if (
    replacing &&
    !window.confirm(t("dialogs.replaceStructure"))
  ) {
    return;
  }
  if (await storyStructureStore.applySelected(replacing)) {
    await manuscriptStore.loadTree();
  }
}

async function selectStructureTemplate(templateId: string) {
  if (!await flushBeatSave()) return;
  storyStructureStore.selectTemplate(templateId);
}

async function generateActiveStructureManuscript() {
  if (await storyStructureStore.generateManuscript()) {
    await manuscriptStore.loadTree();
  }
}

const workspaceStyle = computed(() => ({
  gridTemplateColumns: `${workspace.layout.navigator}px 6px minmax(360px, 1fr) 6px ${workspace.layout.sidebar}px`,
}));

const sceneEditorStyle = computed(() => {
  const fontFamily = projectSettingsDraft.editorFontFamily;
  const fontSize = Number(projectSettingsDraft.editorFontSize);

  return {
    fontFamily: editorFontStacks[fontFamily] ?? editorFontStacks.serif,
    fontSize: `${editorFontSizeOptions.includes(fontSize) ? fontSize : 19}px`,
  };
});

const saveState = computed(() => {
  if (manuscriptStore.error) {
    return ui("Fehler");
  }

  if (manuscriptStore.isSaving) {
    return ui("Speichert");
  }

  return ui("Gespeichert");
});

const projectCopyState = computed(() => {
  if (projectStore.copyError) {
    return formatUiError(projectStore.copyError);
  }

  if (projectStore.isSavingCopy) {
    return ui("Sichert Backup");
  }

  if (projectStore.lastCopyPath) {
    return ui("Backup erstellt");
  }

  return "";
});

const bibleSaveState = computed(() => {
  if (projectBibleStore.error) {
    return ui("Fehler");
  }

  if (projectBibleStore.isSaving) {
    return ui("Speichert");
  }

  return ui("Gespeichert");
});
const worldbuildingSaveState = computed(() => {
  if (worldbuildingStore.error) return ui("Fehler");
  if (worldbuildingStore.isSaving) return ui("Speichert");
  if (worldSystemSavePending.value) return ui("Änderungen ausstehend");
  return ui("Gespeichert");
});

const selectedCard = computed(() => projectBibleStore.selectedCard);
const selectedCardKey = computed(
  () => `${projectBibleStore.activeCardType}:${projectBibleStore.selectedCardId}`,
);
const selectedCardSceneIds = computed(
  () => new Set(projectBibleStore.selectedCardLinks.map((link) => link.sceneId)),
);

const selectedCardTypeLabel = computed(() => cardTypeLabel(projectBibleStore.activeCardType));
const selectedWorldTemplate = computed(() => worldbuildingStore.templates.find((template) => template.id === worldSystemDraft.templateId) ?? worldbuildingStore.templates.find((template) => template.id === "general") ?? null);
const worldSystemNameMissing = computed(() => Boolean(worldbuildingStore.selectedSystem) && !worldSystemDraft.name.trim());
const openWorldSystemFields = computed(() => worldbuildingStore.selectedSystem ? worldSystemOptionalFields.value.filter((field) => !String(worldSystemDraft[field.key] ?? "").trim()) : []);
const sceneWorldSystems = computed(() => worldbuildingStore.systems.filter((system) => projectBibleStore.snapshot?.sceneLinks.some((link) => link.sceneId === manuscriptStore.activeSceneId && link.targetType === "world_system" && link.targetId === system.id)));
const selectedCardWorldSystems = computed(() => worldbuildingStore.systems.filter((system) => projectBibleStore.selectedCardRelationships.some((relation) => (relation.sourceType === "world_system" && relation.sourceId === system.id) || (relation.targetType === "world_system" && relation.targetId === system.id))));
const worldOverviewSystems = computed(() => workspace.activeSection === "writing" ? sceneWorldSystems.value : workspace.activeSection === "bible" ? selectedCardWorldSystems.value : worldbuildingStore.systems);
const relationshipTypes = computed(() => relationshipDefinitions.map((definition) => ({ ...definition, label: t(definition.labelKey) })));
const relationshipEndpoints = computed(() => {
  const existing = projectBibleStore.snapshot?.relationships.find((item) => item.id === relationshipEditId.value);
  if (existing) return { sourceType: existing.sourceType, targetType: existing.targetType };
  if (workspace.activeSection === "worldbuilding") return { sourceType: "world_system" as const, targetType: relationshipTargetType.value };
  return { sourceType: selectedCard.value?.targetType ?? "character", targetType: relationshipTargetType.value };
});
const availableRelationshipTypes = computed(() => relationshipTypes.value.filter((type) => isRelationshipTypeAllowed(relationshipEndpoints.value.sourceType, relationshipEndpoints.value.targetType, type.id)));
const relationshipCandidates = computed(() => projectBibleCardsForType(relationshipTargetType.value)
  .filter((card) => !shouldExcludeRelationshipCandidate(
    card,
    workspace.activeSection,
    selectedCard.value,
    worldbuildingStore.selectedSystemId,
  ))
  .filter((card) => `${card.title} ${card.subtitle}`.toLocaleLowerCase(localeCode()).includes(relationshipSearch.value.trim().toLocaleLowerCase(localeCode()))));

const currentSceneTextHistory = computed(() =>
  sceneDraft.id ? sceneTextHistories[sceneDraft.id] : undefined,
);
const canUndoSceneText = computed(() => Boolean(currentSceneTextHistory.value?.undo.length));
const canRedoSceneText = computed(() => Boolean(currentSceneTextHistory.value?.redo.length));

const writingBibleSceneLinks = computed(
  () =>
    projectBibleStore.snapshot?.sceneLinks.filter(
      (link) => link.sceneId === manuscriptStore.activeSceneId,
    ) ?? [],
);

const writingBibleGroups = computed(() => {
  const groupedCards: Record<ProjectBibleCardType, ProjectBibleCard[]> = {
    character: [],
    location: [],
    item: [],
    note: [],
    faction: [],
    event: [],
  };

  for (const link of writingBibleSceneLinks.value) {
    const card = findProjectBibleCard(link.targetType, link.targetId);
    if (card && card.targetType !== "world_system" && !groupedCards[card.targetType].some((item) => item.id === card.id)) {
      groupedCards[card.targetType].push(card);
    }
  }

  return projectBibleCardTypes.value
    .map((cardType) => ({
      type: cardType.id,
      label: cardType.label,
      cards: groupedCards[cardType.id],
    }))
    .filter((group) => group.cards.length > 0);
});

const writingBibleLinkedCardKeys = computed(
  () => new Set(writingBibleSceneLinks.value.map((link) => `${link.targetType}:${link.targetId}`)),
);

const writingBibleAvailableCards = computed(() => {
  const query = writingBibleSearchQuery.value.trim().toLocaleLowerCase(localeCode());

  return projectBibleCardsForType(writingBibleAddType.value).filter((card) => {
    if (writingBibleLinkedCardKeys.value.has(projectBibleCardKey(card))) {
      return false;
    }

    if (!query) {
      return true;
    }

    return [
      card.title,
      card.subtitle,
      "role" in card ? card.role : "",
      "description" in card ? card.description : "",
      "notes" in card ? card.notes : "",
      "body" in card ? card.body : "",
    ]
      .join(" ")
      .toLocaleLowerCase(localeCode())
      .includes(query);
  });
});

const firstWritingBibleCardKey = computed(() => {
  const firstCard = writingBibleGroups.value[0]?.cards[0];
  return firstCard ? projectBibleCardKey(firstCard) : "";
});

const aiContextPreview = computed(() => {
  const linkedCards = writingBibleGroups.value.reduce((count, group) => count + group.cards.length, 0);

  return [
    sceneDraft.title || ui("Aktuelle Szene"),
    manuscriptStore.activeLocation,
    sceneDraft.summary || ui("Keine Kurzbeschreibung"),
    t("counts.contextCards", linkedCards),
  ];
});

const canRunSceneSummary = computed(
  () =>
    Boolean(sceneDraft.id && sceneDraft.text.trim()) &&
    aiStore.isProviderReachable &&
    aiStore.isConfiguredModelAvailable &&
    !aiStore.isRunning,
);

const canRunSceneContradictions = computed(
  () =>
    Boolean(sceneDraft.id && sceneDraft.text.trim() && writingBibleSceneLinks.value.length) &&
    aiStore.isProviderReachable &&
    aiStore.isConfiguredModelAvailable &&
    !aiStore.isRunning,
);

function sceneSuggestionForAction(actionType: "summarize_scene" | "check_scene_contradictions") {
  return (
    aiStore.suggestions.find(
      (suggestion) =>
        suggestion.id === aiStore.activeSuggestionId && suggestion.actionType === actionType,
    ) ??
    aiStore.suggestions.find(
      (suggestion) => suggestion.actionType === actionType && suggestion.status === "draft",
    ) ??
    aiStore.suggestions.find((suggestion) => suggestion.actionType === actionType) ??
    null
  );
}

const activeSceneSummarySuggestion = computed(() => sceneSuggestionForAction("summarize_scene"));

const activeSceneContradictionSuggestion = computed(() =>
  sceneSuggestionForAction("check_scene_contradictions"),
);

const selectedSceneText = computed(() => sceneTextSelection.text.trim());
const selectedSceneTextLabel = computed(() =>
  selectedSceneText.value.length > 180
    ? `${selectedSceneText.value.slice(0, 180)}...`
    : selectedSceneText.value,
);
const canRunTextAlternatives = computed(
  () =>
    Boolean(sceneDraft.id && selectedSceneText.value) &&
    sceneTextSelection.sceneId === sceneDraft.id &&
    aiStore.isProviderReachable &&
    aiStore.isConfiguredModelAvailable &&
    !aiStore.isRunningAlternatives,
);
const textImprovementMode = computed<ImproveTextMode>(() =>
  selectedSceneText.value && sceneTextSelection.sceneId === sceneDraft.id ? "selection" : "scene",
);
const canRunTextImprovements = computed(
  () =>
    Boolean(sceneDraft.id) &&
    Boolean(textImprovementMode.value === "selection" ? selectedSceneText.value : sceneDraft.text.trim()) &&
    aiStore.isProviderReachable &&
    aiStore.isConfiguredModelAvailable &&
    !aiStore.isRunningImprovements,
);

const canRunBibleAi = computed(
  () =>
    Boolean(bibleAiKeywords.value.trim()) &&
    aiStore.isProviderReachable &&
    aiStore.isConfiguredModelAvailable &&
    !aiStore.isRunningBible &&
    (bibleAiMode.value === "generate" || Boolean(selectedCard.value?.id)),
);

const bibleAiHasDraft = computed(() => {
  return Boolean(Object.entries(bibleAiDraft).some(([key, value]) => key !== "noteType" && String(value).trim()));
});

const bibleAiChangedFields = computed(() => {
  const suggestion = aiStore.activeBibleSuggestion;
  if (!suggestion || suggestion.mode !== "edit" || !suggestion.sourceDraft) return [];
  const fields: Array<{ key: AiBibleEditableField; label: string; multiline: boolean }> =
    suggestion.targetType === "character"
      ? [
          { key: "name", label: ui("Name"), multiline: false }, { key: "role", label: ui("Rolle"), multiline: false },
          { key: "description", label: ui("Beschreibung"), multiline: true }, { key: "notes", label: ui("Notizen"), multiline: true },
        ]
      : suggestion.targetType === "note"
        ? [
            { key: "title", label: ui("Titel"), multiline: false }, { key: "noteType", label: ui("Notiztyp"), multiline: false }, { key: "body", label: ui("Inhalt"), multiline: true },
          ]
        : suggestion.targetType === "faction"
          ? [{ key: "name", label: ui("Name"), multiline: false }, { key: "role", label: ui("Art"), multiline: false }, { key: "description", label: ui("Beschreibung"), multiline: true }, { key: "body", label: ui("Ziele"), multiline: true }, { key: "notes", label: ui("Notizen"), multiline: true }]
          : suggestion.targetType === "event"
            ? [{ key: "title", label: ui("Titel"), multiline: false }, { key: "role", label: ui("Zeitangabe"), multiline: false }, { key: "description", label: ui("Zusammenfassung"), multiline: true }, { key: "body", label: ui("Verlauf / Ausgang"), multiline: true }, { key: "notes", label: ui("Notizen"), multiline: true }]
            : [
            { key: "name", label: ui("Name"), multiline: false }, { key: "description", label: ui("Beschreibung"), multiline: true }, { key: "notes", label: ui("Notizen"), multiline: true },
          ];
  return fields.filter(({ key }) => suggestion.sourceDraft?.[key] !== suggestion.draft[key]);
});

const canSaveProjectSettings = computed(
  () =>
    Boolean(projectSettingsDraft.name.trim()) &&
    Boolean(projectSettingsDraft.defaultLanguage.trim()) &&
    !projectStore.isLoading,
);

const bibleAiDuplicateName = computed(() => {
  if (bibleAiMode.value !== "generate") {
    return false;
  }

  const title = cardDraft.targetType === "note" ? bibleAiDraft.title : bibleAiDraft.name;
  const normalized = title.trim().toLocaleLowerCase(localeCode());
  if (!normalized) {
    return false;
  }

  return projectBibleStore.cards.some((card) => card.title.trim().toLocaleLowerCase(localeCode()) === normalized);
});

const aiProviderStateLabel = computed(() => {
  if (aiStore.isCheckingProvider) {
    return ui("Prüft lokale KI");
  }

  return aiStore.providerStatus ? t(`provider.${aiStore.providerStatus.code}`) : ui("KI-Status noch nicht geprüft.");
});

const selectedAiModelInfo = computed(() => {
  const modelName = aiStore.settings?.model ?? "";

  return aiStore.models.find((model) => model.name === modelName || model.id === modelName) ?? null;
});

const aiConfiguredModelLabel = computed(() => {
  if (selectedAiModelInfo.value) {
    return modelOptionLabel(selectedAiModelInfo.value);
  }

  return aiStore.settings?.model || ui("Kein Modell ausgewählt");
});

const aiModelMissing = computed(
  () =>
    aiStore.isProviderReachable &&
    !aiStore.isLoadingModels &&
    aiStore.hasModels &&
    Boolean(aiStore.settings?.model) &&
    !aiStore.isConfiguredModelAvailable,
);

const aiModelListEmpty = computed(
  () => aiStore.isProviderReachable && !aiStore.isLoadingModels && !aiStore.hasModels,
);

function clampPanelWidth(panel: LayoutPanel, width: number) {
  const bounds = panelBounds[panel];

  return Math.min(bounds.max, Math.max(bounds.min, Math.round(width)));
}

function loadStoredLayout() {
  const rawLayout = window.localStorage.getItem(layoutStorageKey);

  if (!rawLayout) {
    return;
  }

  try {
    const parsed = JSON.parse(rawLayout) as Partial<WorkspaceLayout>;
    workspace.setLayout({
      navigator: clampPanelWidth("navigator", parsed.navigator ?? defaultWorkspaceLayout.navigator),
      sidebar: clampPanelWidth("sidebar", parsed.sidebar ?? defaultWorkspaceLayout.sidebar),
    });
  } catch {
    window.localStorage.removeItem(layoutStorageKey);
  }
}

function navigatorCollapseStorageKey(path = projectStore.activeProject?.path) {
  return path ? `${navigatorCollapseStorageKeyPrefix}:${encodeURIComponent(path)}` : "";
}

function loadStoredNavigatorCollapse(path = projectStore.activeProject?.path) {
  const key = navigatorCollapseStorageKey(path);

  if (!key) {
    collapsedAreaIds.value = new Set();
    collapsedChapterIds.value = new Set();
    return;
  }

  const rawState = window.localStorage.getItem(key);
  if (!rawState) {
    collapsedAreaIds.value = new Set();
    collapsedChapterIds.value = new Set();
    return;
  }

  try {
    const parsed = JSON.parse(rawState) as { areas?: unknown; chapters?: unknown };
    collapsedAreaIds.value = new Set(
      Array.isArray(parsed.areas) ? parsed.areas.filter((id): id is string => typeof id === "string") : [],
    );
    collapsedChapterIds.value = new Set(
      Array.isArray(parsed.chapters)
        ? parsed.chapters.filter((id): id is string => typeof id === "string")
        : [],
    );
  } catch {
    window.localStorage.removeItem(key);
    collapsedAreaIds.value = new Set();
    collapsedChapterIds.value = new Set();
  }
}

function saveStoredNavigatorCollapse() {
  const key = navigatorCollapseStorageKey();

  if (!key) {
    return;
  }

  window.localStorage.setItem(
    key,
    JSON.stringify({
      areas: [...collapsedAreaIds.value],
      chapters: [...collapsedChapterIds.value],
    }),
  );
}

function pruneNavigatorCollapse() {
  const areaIds = new Set(manuscriptStore.areas.map((area) => area.id));
  const chapterIds = new Set(manuscriptStore.chapters.map((chapter) => chapter.id));
  const nextAreaIds = new Set([...collapsedAreaIds.value].filter((areaId) => areaIds.has(areaId)));
  const nextChapterIds = new Set(
    [...collapsedChapterIds.value].filter((chapterId) => chapterIds.has(chapterId)),
  );

  if (
    nextAreaIds.size !== collapsedAreaIds.value.size ||
    nextChapterIds.size !== collapsedChapterIds.value.size
  ) {
    collapsedAreaIds.value = nextAreaIds;
    collapsedChapterIds.value = nextChapterIds;
    saveStoredNavigatorCollapse();
  }
}

function isAreaCollapsed(areaId: string) {
  return collapsedAreaIds.value.has(areaId);
}

function isChapterCollapsed(chapterId: string) {
  return collapsedChapterIds.value.has(chapterId);
}

function toggleAreaCollapsed(areaId: string) {
  const nextIds = new Set(collapsedAreaIds.value);

  if (nextIds.has(areaId)) {
    nextIds.delete(areaId);
  } else {
    nextIds.add(areaId);
  }

  collapsedAreaIds.value = nextIds;
  saveStoredNavigatorCollapse();
}

function toggleChapterCollapsed(chapterId: string) {
  const nextIds = new Set(collapsedChapterIds.value);

  if (nextIds.has(chapterId)) {
    nextIds.delete(chapterId);
  } else {
    nextIds.add(chapterId);
  }

  collapsedChapterIds.value = nextIds;
  saveStoredNavigatorCollapse();
}

function expandArea(areaId: string) {
  if (!collapsedAreaIds.value.has(areaId)) {
    return;
  }

  const nextIds = new Set(collapsedAreaIds.value);
  nextIds.delete(areaId);
  collapsedAreaIds.value = nextIds;
  saveStoredNavigatorCollapse();
}

function expandChapter(chapterId: string) {
  if (!collapsedChapterIds.value.has(chapterId)) {
    return;
  }

  const nextIds = new Set(collapsedChapterIds.value);
  nextIds.delete(chapterId);
  collapsedChapterIds.value = nextIds;
  saveStoredNavigatorCollapse();
}

function expandChapterPath(chapterId: string) {
  const area = manuscriptStore.areas.find((item) =>
    item.chapters.some((chapter) => chapter.id === chapterId),
  );

  if (area) {
    expandArea(area.id);
  }

  expandChapter(chapterId);
}

function expandActiveLocation() {
  const activeChapter = manuscriptStore.activeChapter;

  if (activeChapter) {
    expandChapterPath(activeChapter.id);
  }
}

function startPanelResize(panel: LayoutPanel, event: PointerEvent) {
  event.preventDefault();

  const startX = event.clientX;
  const startWidth = workspace.layout[panel];
  const direction = panel === "sidebar" ? -1 : 1;

  function resize(moveEvent: PointerEvent) {
    const nextWidth = startWidth + (moveEvent.clientX - startX) * direction;
    workspace.setPanelWidth(panel, clampPanelWidth(panel, nextWidth));
  }

  function stopResize() {
    window.removeEventListener("pointermove", resize);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
  }

  window.addEventListener("pointermove", resize);
  window.addEventListener("pointerup", stopResize, { once: true });
  window.addEventListener("pointercancel", stopResize, { once: true });
}

function ensureSceneTextHistory(sceneId: string, text: string) {
  if (!sceneTextHistories[sceneId]) {
    sceneTextHistories[sceneId] = {
      undo: [],
      redo: [],
      current: text,
    };
  }

  sceneTextHistories[sceneId].current = text;
  return sceneTextHistories[sceneId];
}

function clearAllSceneTextHistories() {
  for (const sceneId of Object.keys(sceneTextHistories)) {
    delete sceneTextHistories[sceneId];
  }
}

function recordSceneTextHistoryChange(sceneId: string, oldText: string, newText: string) {
  if (!sceneId || oldText === newText) {
    return;
  }

  const history = sceneTextHistories[sceneId] ?? ensureSceneTextHistory(sceneId, oldText);
  const previous = history.current === oldText ? oldText : history.current;
  if (previous !== newText && history.undo[history.undo.length - 1] !== previous) {
    history.undo.push(previous);
    if (history.undo.length > sceneTextHistoryLimit) {
      history.undo.shift();
    }
  }

  history.current = newText;
  history.redo = [];
}

async function applySceneTextFromHistory(text: string) {
  isApplyingTextHistory.value = true;
  sceneDraft.text = text;
  clearSceneTextSelection();
  await nextTick();
  isApplyingTextHistory.value = false;
  sceneTextAreaRef.value?.focus();
  await persistSceneDraftNow();
}

async function undoSceneText() {
  const history = currentSceneTextHistory.value;
  if (!sceneDraft.id || !history?.undo.length) {
    return;
  }

  const previous = history.undo.pop();
  if (previous === undefined) {
    return;
  }

  history.redo.push(sceneDraft.text);
  history.current = previous;
  await applySceneTextFromHistory(previous);
}

async function redoSceneText() {
  const history = currentSceneTextHistory.value;
  if (!sceneDraft.id || !history?.redo.length) {
    return;
  }

  const next = history.redo.pop();
  if (next === undefined) {
    return;
  }

  history.undo.push(sceneDraft.text);
  if (history.undo.length > sceneTextHistoryLimit) {
    history.undo.shift();
  }
  history.current = next;
  await applySceneTextFromHistory(next);
}

function hydrateSceneDraft(scene: SceneSnapshot | null) {
  if (
    scene?.id === sceneDraft.id &&
    (sceneAutosave.saving.value || sceneAutosave.pending.value)
  ) {
    return;
  }
  isHydratingSceneDraft.value = true;
  sceneAutosave.reset();
  sceneDraft.id = scene?.id ?? "";
  sceneDraft.title = scene?.title ?? "";
  sceneDraft.status = scene?.status ?? "draft";
  sceneDraft.summary = scene?.summary ?? "";
  sceneDraft.text = scene?.text ?? "";
  if (scene?.id) {
    ensureSceneTextHistory(scene.id, scene.text);
  }
  window.setTimeout(() => {
    isHydratingSceneDraft.value = false;
  });
}

function hydrateCardDraft(card: ProjectBibleCard | null) {
  isHydratingCardDraft.value = true;
  cardAutosave.reset();
  cardDraft.id = card?.id ?? "";
  cardDraft.targetType = card?.targetType ?? projectBibleStore.activeCardType;
  cardDraft.name = card && "name" in card ? card.name : "";
  cardDraft.role = card ? ("role" in card ? card.role : "factionType" in card ? card.factionType : "timeLabel" in card ? card.timeLabel : "") : "";
  cardDraft.description = card ? ("description" in card ? card.description : "summary" in card ? card.summary : "") : "";
  cardDraft.notes = card && "notes" in card ? card.notes : "";
  cardDraft.title = card && "title" in card ? card.title : "";
  cardDraft.body = card ? ("body" in card ? card.body : "goals" in card ? card.goals : "outcome" in card ? card.outcome : "") : "";
  cardDraft.noteType = card && "noteType" in card ? card.noteType : "general";
  window.setTimeout(() => {
    isHydratingCardDraft.value = false;
  });
}

function hydrateProjectSettings() {
  const project = projectStore.activeProject;
  projectSettingsDraft.name = project?.name ?? "";
  projectSettingsDraft.description = project?.description ?? "";
  projectSettingsDraft.defaultLanguage = project?.default_language ?? "de";
  projectSettingsDraft.editorFontFamily = normalizeEditorFontFamily(project?.editor_font_family);
  projectSettingsDraft.editorFontSize = normalizeEditorFontSize(project?.editor_font_size);
}

function normalizeEditorFontFamily(fontFamily?: string): EditorFontFamily {
  if (fontFamily === "sans" || fontFamily === "monospace") {
    return fontFamily;
  }

  return "serif";
}

function normalizeEditorFontSize(fontSize?: number): number {
  return fontSize && editorFontSizeOptions.includes(fontSize) ? fontSize : 19;
}

function formatProjectTimestamp(value?: string) {
  const seconds = Number(value ?? 0);
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return "-";
  }

  return new Intl.DateTimeFormat(locale.value === "en" ? "en-GB" : "de-DE", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(seconds * 1000));
}

function hydrateBibleAiDraft(suggestion = aiStore.activeBibleSuggestion) {
  const draft = suggestion?.draft;
  bibleAiDraft.name = draft?.name ?? "";
  bibleAiDraft.role = draft?.role ?? "";
  bibleAiDraft.description = draft?.description ?? "";
  bibleAiDraft.notes = draft?.notes ?? "";
  bibleAiDraft.title = draft?.title ?? "";
  bibleAiDraft.body = draft?.body ?? "";
  bibleAiDraft.noteType = draft?.noteType ?? "general";
  if (suggestion?.mode === "edit" && suggestion.sourceDraft) {
    const allowed: AiBibleEditableField[] = suggestion.targetType === "character"
      ? ["name", "role", "description", "notes"]
      : suggestion.targetType === "note"
        ? ["title", "noteType", "body"]
        : suggestion.targetType === "faction"
          ? ["name", "role", "description", "body", "notes"]
          : suggestion.targetType === "event"
            ? ["title", "role", "description", "body", "notes"]
        : ["name", "description", "notes"];
    bibleAiSelectedFields.value = allowed.filter((key) => suggestion.sourceDraft?.[key] !== draft?.[key]);
  } else {
    bibleAiSelectedFields.value = [];
  }
}

function hydrateWorldSystemDraft() {
  isHydratingWorldSystemDraft.value = true;
  worldSystemAutosave.reset();
  const system = worldbuildingStore.selectedSystem;
  Object.assign(worldSystemDraft, system ? { id: system.id, name: system.name, category: system.category, templateId: system.templateId, principle: system.principle, rules: system.rules, capabilities: system.capabilities, costs: system.costs, limits: system.limits, access: system.access, distribution: system.distribution, worldImpact: system.worldImpact, openQuestions: system.openQuestions, notes: system.notes } : { id: "", name: "", category: "magic", templateId: "magic", principle: "", rules: "", capabilities: "", costs: "", limits: "", access: "", distribution: "", worldImpact: "", openQuestions: "", notes: "" });
  window.setTimeout(() => { isHydratingWorldSystemDraft.value = false; });
}

function scheduleWorldSystemSave() {
  if (isHydratingWorldSystemDraft.value || !worldSystemDraft.id) return;
  worldSystemAutosave.schedule({ ...worldSystemDraft });
}

async function flushWorldSystemDraftSave(): Promise<boolean> {
  return worldSystemAutosave.flush();
}

async function persistWorldSystemDraftNow(): Promise<boolean> {
  return flushWorldSystemDraftSave();
}

async function selectWorldSystem(id: string) {
  if (id === worldbuildingStore.selectedSystemId) return;
  if (!await persistWorldSystemDraftNow()) return;
  worldbuildingStore.select(id);
}

function defaultTemplateForCategory(category: WorldSystemCategory) { return ["magic", "technology", "psionics", "hybrid"].includes(category) ? category : "general"; }
function worldCategoryLabel(category: string) { return worldSystemCategories.value.find((item) => item.id === category)?.label ?? category; }
function openNewWorldSystemDialog() { newWorldSystemName.value = ""; newWorldSystemCategory.value = "magic"; newWorldSystemTemplateId.value = "magic"; showWorldSystemDialog.value = true; }
async function createWorldSystemFromDialog() { const created = await worldbuildingStore.create(newWorldSystemName.value, newWorldSystemCategory.value, newWorldSystemTemplateId.value); if (created) { showWorldSystemDialog.value = false; await projectBibleStore.load(); hydrateWorldSystemDraft(); } }
async function openWorldSystem(id: string) { if (id !== worldbuildingStore.selectedSystemId && !await persistWorldSystemDraftNow()) return; worldbuildingStore.select(id); workspace.setActiveSection("worldbuilding"); }
async function archiveWorldSystem() { if (!worldbuildingStore.selectedSystem || !window.confirm(t("dialogs.archiveWorld", { name: worldbuildingStore.selectedSystem.name }))) return; if (!await persistWorldSystemDraftNow()) return; worldSystemAutosave.reset(); await worldbuildingStore.archiveSelected(); await projectBibleStore.load(); hydrateWorldSystemDraft(); }
async function toggleSceneWorldSystem(systemId: string, linked: boolean, sceneId = manuscriptStore.activeSceneId) { if (!sceneId) return; const current = new Set(projectBibleStore.snapshot?.sceneLinks.filter((link) => link.targetType === "world_system" && link.targetId === systemId).map((link) => link.sceneId) ?? []); linked ? current.add(sceneId) : current.delete(sceneId); await projectBibleStore.replaceCardSceneLinks("world_system", systemId, [...current]); }
async function linkSelectedCardToWorld(systemId: string) { const card = selectedCard.value; if (!card) return; await projectBibleStore.saveRelationship({ sourceType: card.targetType, sourceId: card.id, targetType: "world_system", targetId: systemId, relationshipType: "uses_system", description: "" }); }
async function addWorldRelationship() { if (!worldbuildingStore.selectedSystemId || !relationshipTargetId.value) return; const existing = projectBibleStore.snapshot?.relationships.find((item) => item.id === relationshipEditId.value); if (await projectBibleStore.saveRelationship({ id: existing?.id, sourceType: existing?.sourceType ?? "world_system", sourceId: existing?.sourceId ?? worldbuildingStore.selectedSystemId, targetType: existing?.targetType ?? relationshipTargetType.value, targetId: existing?.targetId ?? relationshipTargetId.value, relationshipType: relationshipType.value, description: relationshipDescription.value })) { relationshipEditId.value = ""; relationshipTargetId.value = ""; relationshipDescription.value = ""; relationshipSearch.value = ""; } }

function scheduleSceneSave() {
  if (isHydratingSceneDraft.value || isApplyingTextHistory.value || !sceneDraft.id) {
    return;
  }

  sceneAutosave.schedule({
    id: sceneDraft.id,
    title: sceneDraft.title,
    status: sceneDraft.status,
    summary: sceneDraft.summary,
    text: sceneDraft.text,
  });
}

async function persistSceneDraftNow(): Promise<boolean> {
  if (isHydratingSceneDraft.value || !sceneDraft.id) {
    return true;
  }
  return sceneAutosave.flush();
}

function scheduleCardSave() {
  if (isHydratingCardDraft.value || !cardDraft.id) {
    return;
  }

  cardAutosave.schedule(currentCardDraftInput());
}

function currentCardDraftInput(): ProjectBibleCardUpdate {
  if (cardDraft.targetType === "note") {
    return {
      targetType: "note",
      id: cardDraft.id,
      title: cardDraft.title,
      body: cardDraft.body,
      noteType: cardDraft.noteType,
    };
  }

  if (cardDraft.targetType === "event") return { targetType: "event", id: cardDraft.id, title: cardDraft.title, role: cardDraft.role, description: cardDraft.description, body: cardDraft.body, notes: cardDraft.notes };
  if (cardDraft.targetType === "faction") return { targetType: "faction", id: cardDraft.id, name: cardDraft.name, role: cardDraft.role, description: cardDraft.description, body: cardDraft.body, notes: cardDraft.notes };

  return {
    targetType: cardDraft.targetType,
    id: cardDraft.id,
    name: cardDraft.name,
    role: cardDraft.role,
    description: cardDraft.description,
    notes: cardDraft.notes,
  };
}

async function flushCardDraftSave(): Promise<boolean> {
  return cardAutosave.flush();
}

async function persistCardDraftNow(): Promise<boolean> {
  return flushCardDraftSave();
}

function cardTypeLabel(type: string) {
  return projectBibleCardTypes.value.find((cardType) => cardType.id === type)?.label ?? t("bible.cards");
}

function cardTypeSingular(type: string) {
  return ui(({ character: "Figur", location: "Ort", item: "Gegenstand", faction: "Fraktion", event: "Ereignis", world_system: "Weltsystem", note: "Notiz" } as Record<string, string>)[type] ?? "Notiz");
}

function noteTypeLabel(type: string) {
  return noteTypes.value.find((noteType) => noteType.id === type)?.label ?? type;
}

function modelOptionLabel(model: { name: string; size: number }) {
  if (!model.size) {
    return model.name;
  }

  return `${model.name} (${formatModelSize(model.size)})`;
}

function formatModelSize(size: number) {
  if (size >= 1024 * 1024 * 1024) {
    return `${new Intl.NumberFormat(localeCode(), { maximumFractionDigits: 1 }).format(size / 1024 / 1024 / 1024)} GB`;
  }

  if (size >= 1024 * 1024) {
    return `${new Intl.NumberFormat(localeCode()).format(Math.round(size / 1024 / 1024))} MB`;
  }

  return `${size} B`;
}

function projectBibleCardKey(card: Pick<KnowledgeCard, "targetType" | "id">) {
  return `${card.targetType}:${card.id}`;
}

function isWritingBibleCardOpen(card: ProjectBibleCard) {
  return writingBibleOpenCardKey.value === projectBibleCardKey(card);
}

async function toggleWritingBibleCard(card: ProjectBibleCard) {
  const key = projectBibleCardKey(card);
  if (writingBibleOpenCardKey.value === key) {
    if (!await persistCardDraftNow()) return;
    writingBibleOpenCardKey.value = "";
    return;
  }

  if (!await persistCardDraftNow()) return;
  projectBibleStore.setActiveCardType(card.targetType);
  projectBibleStore.selectCard(card.id);
  writingBibleOpenCardKey.value = key;
}

function projectBibleCardsForType(targetType: ProjectBibleCardType): ProjectBibleCard[];
function projectBibleCardsForType(targetType: "world_system"): KnowledgeCard[];
function projectBibleCardsForType(targetType: KnowledgeTargetType): KnowledgeCard[];
function projectBibleCardsForType(targetType: KnowledgeTargetType): KnowledgeCard[] {
  if (targetType === "world_system") return worldbuildingStore.systems.map((card) => ({ ...card, targetType: "world_system", title: card.name, subtitle: worldCategoryLabel(card.category) }));
  const snapshot = projectBibleStore.snapshot;
  if (!snapshot) {
    return [];
  }

  if (targetType === "character") {
    return snapshot.characters.map((card) => ({
      ...card,
      targetType: "character",
      title: card.name,
      subtitle: card.role,
    }));
  }

  if (targetType === "location") {
    return snapshot.locations.map((card) => ({
      ...card,
      targetType: "location",
      title: card.name,
      subtitle: card.description,
    }));
  }

  if (targetType === "item") {
    return snapshot.items.map((card) => ({
      ...card,
      targetType: "item",
      title: card.name,
      subtitle: card.description,
    }));
  }

  if (targetType === "faction") return snapshot.factions.map((card) => ({ ...card, targetType: "faction", title: card.name, subtitle: card.factionType || card.description }));
  if (targetType === "event") return snapshot.events.map((card) => ({ ...card, targetType: "event", name: card.title, subtitle: card.timeLabel || card.summary }));
  return snapshot.notes.map((card) => ({
    ...card,
    targetType: "note",
    name: card.title,
    subtitle: noteTypeLabel(card.noteType),
  }));
}

function findProjectBibleCard(
  targetType: KnowledgeTargetType,
  targetId: string,
): KnowledgeCard | null {
  if (targetType === "world_system") { const card = worldbuildingStore.systems.find((item) => item.id === targetId); return card ? { ...card, targetType: "world_system", title: card.name, subtitle: worldCategoryLabel(card.category) } : null; }
  const snapshot = projectBibleStore.snapshot;
  if (!snapshot) {
    return null;
  }

  if (targetType === "character") {
    const card = snapshot.characters.find((item) => item.id === targetId);
    return card
      ? {
          ...card,
          targetType: "character",
          title: card.name,
          subtitle: card.role,
        }
      : null;
  }

  if (targetType === "location") {
    const card = snapshot.locations.find((item) => item.id === targetId);
    return card
      ? {
          ...card,
          targetType: "location",
          title: card.name,
          subtitle: card.description,
        }
      : null;
  }

  if (targetType === "item") {
    const card = snapshot.items.find((item) => item.id === targetId);
    return card
      ? {
          ...card,
          targetType: "item",
          title: card.name,
          subtitle: card.description,
        }
      : null;
  }

  if (targetType === "faction") {
    const card = snapshot.factions.find((item) => item.id === targetId);
    return card ? { ...card, targetType: "faction", title: card.name, subtitle: card.factionType || card.description } : null;
  }
  if (targetType === "event") {
    const card = snapshot.events.find((item) => item.id === targetId);
    return card ? { ...card, targetType: "event", name: card.title, subtitle: card.timeLabel || card.summary } : null;
  }
  const card = snapshot.notes.find((item) => item.id === targetId);
  return card
    ? {
        ...card,
        targetType: "note",
        name: card.title,
        subtitle: noteTypeLabel(card.noteType),
      }
    : null;
}

function bibleCardNavigatorKey(cardId: string) {
  return `bible-card:${cardId}`;
}

async function selectBibleCard(card: ProjectBibleCard) {
  if (card.id === projectBibleStore.selectedCardId) return;
  if (!await persistCardDraftNow()) return;
  projectBibleStore.selectCard(card.id);
  focusNavigatorItem(bibleCardNavigatorKey(card.id));
}

async function createBibleCard() {
  if (!await persistCardDraftNow()) return;
  await projectBibleStore.createCard();
}

async function selectBibleCardType(cardType: ProjectBibleCardType) {
  if (cardType === projectBibleStore.activeCardType) return;
  if (!await persistCardDraftNow()) return;
  projectBibleStore.setActiveCardType(cardType);
}

async function createWritingBibleCard() {
  if (!manuscriptStore.activeSceneId) {
    return;
  }

  if (!await persistCardDraftNow()) return;
  const card = await projectBibleStore.createCard(writingBibleAddType.value);
  if (!card) {
    return;
  }

  await projectBibleStore.addSceneLinkToCard(card.targetType, card.id, manuscriptStore.activeSceneId);
  writingBibleOpenCardKey.value = projectBibleCardKey(card);
}

async function addWritingBibleCard(card: ProjectBibleCard) {
  if (!manuscriptStore.activeSceneId) {
    return;
  }

  if (!await persistCardDraftNow()) return;
  await projectBibleStore.addSceneLinkToCard(card.targetType, card.id, manuscriptStore.activeSceneId);
  writingBibleOpenCardKey.value = projectBibleCardKey(card);
}

async function archiveSelectedBibleCard() {
  const card = selectedCard.value;
  if (!card) {
    return;
  }

  if (window.confirm(t("dialogs.archiveCard", { type: cardTypeSingular(card.targetType), name: card.title }))) {
    if (!await persistCardDraftNow()) return;
    cardAutosave.reset();
    await projectBibleStore.deleteSelectedCard();
  }
}

function isSceneLinkedToSelectedCard(sceneId: string) {
  return selectedCardSceneIds.value.has(sceneId);
}

async function toggleSelectedCardScene(sceneId: string, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  const sceneIds = new Set(selectedCardSceneIds.value);

  if (checked) {
    sceneIds.add(sceneId);
  } else {
    sceneIds.delete(sceneId);
  }

  await projectBibleStore.replaceSelectedCardSceneLinks([...sceneIds]);
}

async function selectScene(sceneId: string): Promise<boolean> {
  if (sceneId === manuscriptStore.activeSceneId) return true;
  if (!await persistSceneDraftNow()) return false;
  manuscriptStore.selectScene(sceneId);
  return true;
}

async function selectChapter(chapter: ChapterSnapshot) {
  const firstScene = chapter.scenes[0];
  if (firstScene) {
    await selectScene(firstScene.id);
  }
}

async function selectArea(area: AreaSnapshot) {
  const firstScene = area.chapters.find((chapter) => chapter.scenes.length > 0)?.scenes[0];
  if (firstScene) {
    await selectScene(firstScene.id);
  }
}

async function updateAreaTitle(area: AreaSnapshot, event: Event) {
  const input = event.target as HTMLInputElement;
  const title = input.value.trim() || ui("Neuer Bereich");

  if (title === area.title) {
    input.value = area.title;
    return;
  }

  await manuscriptStore.updateArea({
    ...area,
    title,
  });
}

async function updateChapterTitle(chapter: ChapterSnapshot, event: Event) {
  const input = event.target as HTMLInputElement;
  const title = input.value.trim() || ui("Neues Kapitel");

  if (title === chapter.title) {
    input.value = chapter.title;
    return;
  }

  await manuscriptStore.updateChapter({
    ...chapter,
    title,
  });
}

function commitChapterTitle(event: Event) {
  (event.target as HTMLInputElement).blur();
}

function areaNavigatorKey(areaId: string) {
  return `area:${areaId}`;
}

function chapterNavigatorKey(chapterId: string) {
  return `chapter:${chapterId}`;
}

function sceneNavigatorKey(sceneId: string) {
  return `scene:${sceneId}`;
}

function focusNavigatorItem(key: string) {
  focusedNavigatorKey.value = key;
}

function blurNavigatorItem(key: string) {
  if (focusedNavigatorKey.value === key) {
    focusedNavigatorKey.value = "";
  }
}

async function restoreNavigatorFocus(key: string) {
  await nextTick();
  const element = document.querySelector<HTMLElement>(`[data-navigator-key="${key}"]`);
  element?.focus();
}

async function handleAreaKeydown(area: AreaSnapshot, event: KeyboardEvent) {
  if (event.isComposing) {
    return;
  }

  if (event.key !== "ArrowUp" && event.key !== "ArrowDown") {
    return;
  }

  event.preventDefault();
  await moveAreaByOffset(area, event.key === "ArrowUp" ? -1 : 1);
  await restoreNavigatorFocus(areaNavigatorKey(area.id));
}

async function handleChapterKeydownInArea(
  area: AreaSnapshot,
  chapter: ChapterSnapshot,
  event: KeyboardEvent,
) {
  if (event.isComposing) {
    return;
  }

  if (event.key === "ArrowUp") {
    event.preventDefault();
    await moveChapterByOffset(area, chapter, -1);
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    await moveChapterByOffset(area, chapter, 1);
  } else if (event.key === "ArrowLeft") {
    event.preventDefault();
    await moveChapterToAdjacentArea(area, chapter, -1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    await moveChapterToAdjacentArea(area, chapter, 1);
  } else {
    return;
  }

  await restoreNavigatorFocus(chapterNavigatorKey(chapter.id));
}

async function handleSceneKeydown(
  chapter: ChapterSnapshot,
  scene: SceneSnapshot,
  event: KeyboardEvent,
) {
  if (event.isComposing) {
    return;
  }

  if (event.key === "ArrowUp") {
    event.preventDefault();
    await moveSceneByOffset(chapter, scene, -1);
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    await moveSceneByOffset(chapter, scene, 1);
  } else if (event.key === "ArrowLeft") {
    event.preventDefault();
    await moveSceneToAdjacentChapter(chapter, scene, -1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    await moveSceneToAdjacentChapter(chapter, scene, 1);
  } else {
    return;
  }

  await restoreNavigatorFocus(sceneNavigatorKey(scene.id));
}

function areaDropKey(areaId: string) {
  return `area:${areaId}`;
}

function areaEndDropKey(areaId: string) {
  return `area-end:${areaId}`;
}

function chapterDropKey(chapterId: string) {
  return `chapter:${chapterId}`;
}

function sceneDropKey(chapterId: string, sceneIndex: number) {
  return `scene:${chapterId}:${sceneIndex}`;
}

function chapterEndDropKey(chapterId: string) {
  return `chapter-end:${chapterId}`;
}

function readDropTarget(event: PointerEvent): DropTarget | null {
  const element = document.elementFromPoint(event.clientX, event.clientY);
  const target = element?.closest<HTMLElement>("[data-drop-kind]");

  if (!target?.dataset.dropKind) {
    return null;
  }

  if (target.dataset.dropKind === "area") {
    if (!target.dataset.areaId) {
      return null;
    }

    return {
      kind: "area",
      areaId: target.dataset.areaId,
      areaIndex: Number(target.dataset.areaIndex ?? 0),
    };
  }

  if (target.dataset.dropKind === "area-end") {
    if (!target.dataset.areaId) {
      return null;
    }

    return {
      kind: "areaEnd",
      areaId: target.dataset.areaId,
    };
  }

  if (target.dataset.dropKind === "chapter") {
    if (!target.dataset.chapterId) {
      return null;
    }

    return {
      kind: "chapter",
      areaId: target.dataset.areaId ?? "",
      chapterId: target.dataset.chapterId,
      chapterIndex: Number(target.dataset.chapterIndex ?? 0),
    };
  }

  if (target.dataset.dropKind === "scene") {
    if (!target.dataset.chapterId) {
      return null;
    }

    return {
      kind: "scene",
      chapterId: target.dataset.chapterId,
      sceneIndex: Number(target.dataset.sceneIndex ?? 0),
    };
  }

  if (target.dataset.dropKind === "chapter-end") {
    if (!target.dataset.chapterId) {
      return null;
    }

    return {
      kind: "chapterEnd",
      chapterId: target.dataset.chapterId,
    };
  }

  return null;
}

function targetDropKey(target: DropTarget | null) {
  if (!target) {
    return "";
  }

  if (target.kind === "chapter") {
    return chapterDropKey(target.chapterId);
  }

  if (target.kind === "area") {
    return areaDropKey(target.areaId);
  }

  if (target.kind === "areaEnd") {
    return areaEndDropKey(target.areaId);
  }

  if (target.kind === "scene") {
    return sceneDropKey(target.chapterId, target.sceneIndex);
  }

  return chapterEndDropKey(target.chapterId);
}

function startPointerDrag(payload: DragPayload, event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }

  event.preventDefault();
  pointerDrag.value = {
    payload,
    startX: event.clientX,
    startY: event.clientY,
    active: false,
  };
  pointerDropKey.value = "";

  const handle = event.currentTarget as HTMLElement;
  handle.setPointerCapture?.(event.pointerId);
  window.addEventListener("pointermove", movePointerDrag);
  window.addEventListener("pointerup", finishPointerDrag, { once: true });
  window.addEventListener("pointercancel", cancelPointerDrag, { once: true });
}

function movePointerDrag(event: PointerEvent) {
  const state = pointerDrag.value;
  if (!state) {
    return;
  }

  const movedEnough =
    Math.abs(event.clientX - state.startX) > 3 || Math.abs(event.clientY - state.startY) > 3;
  state.active = state.active || movedEnough;
  pointerDropKey.value = state.active ? targetDropKey(readDropTarget(event)) : "";
}

function cancelPointerDrag() {
  pointerDrag.value = null;
  pointerDropKey.value = "";
  window.removeEventListener("pointermove", movePointerDrag);
  window.removeEventListener("pointerup", finishPointerDrag);
  window.removeEventListener("pointercancel", cancelPointerDrag);
}

function finishPointerDrag(event: PointerEvent) {
  const state = pointerDrag.value;
  const target = state?.active ? readDropTarget(event) : null;
  cancelPointerDrag();

  if (state && target) {
    void applyPointerDrop(state.payload, target);
  }
}

async function applyPointerDrop(payload: DragPayload, target: DropTarget) {
  if (payload.type === "area") {
    if (target.kind === "area" && target.areaId !== payload.areaId) {
      await manuscriptStore.moveArea(payload.areaId, target.areaIndex);
    }
    return;
  }

  if (payload.type === "chapter") {
    const targetAreaId =
      target.kind === "chapter" || target.kind === "areaEnd" || target.kind === "area"
        ? target.areaId
        : "";
    const targetChapterIndex =
      target.kind === "chapter"
        ? target.chapterIndex
        : manuscriptStore.areas.find((area) => area.id === targetAreaId)?.chapters.length ?? 0;

    if (targetAreaId && targetChapterIndex >= 0) {
      await manuscriptStore.moveChapter(payload.chapterId, targetAreaId, targetChapterIndex);
      expandArea(targetAreaId);
    }
    return;
  }

  if (target.kind === "scene") {
    await manuscriptStore.moveScene(payload.sceneId, target.chapterId, target.sceneIndex);
    expandChapterPath(target.chapterId);
    return;
  }

  if (target.kind === "chapterEnd") {
    await manuscriptStore.moveScene(payload.sceneId, target.chapterId, Number.MAX_SAFE_INTEGER);
    expandChapterPath(target.chapterId);
    return;
  }

  if (target.kind === "chapter") {
    await manuscriptStore.moveScene(payload.sceneId, target.chapterId, Number.MAX_SAFE_INTEGER);
    expandChapterPath(target.chapterId);
  }
}

function startAreaPointerDrag(area: AreaSnapshot, event: PointerEvent) {
  startPointerDrag({ type: "area", areaId: area.id }, event);
}

function startChapterPointerDrag(chapter: ChapterSnapshot, event: PointerEvent) {
  startPointerDrag({ type: "chapter", chapterId: chapter.id, areaId: chapter.areaId }, event);
}

function startScenePointerDrag(
  chapter: ChapterSnapshot,
  scene: SceneSnapshot,
  event: PointerEvent,
) {
  startPointerDrag({ type: "scene", sceneId: scene.id, chapterId: chapter.id }, event);
}

async function moveAreaByOffset(area: AreaSnapshot, offset: number) {
  const currentIndex = manuscriptStore.areas.findIndex((item) => item.id === area.id);
  if (currentIndex < 0) {
    return;
  }

  const targetIndex = Math.min(
    manuscriptStore.areas.length - 1,
    Math.max(0, currentIndex + offset),
  );
  if (targetIndex !== currentIndex) {
    await manuscriptStore.moveArea(area.id, targetIndex);
  }
}

async function moveChapterByOffset(area: AreaSnapshot, chapter: ChapterSnapshot, offset: number) {
  const currentIndex = area.chapters.findIndex((item) => item.id === chapter.id);
  if (currentIndex < 0) {
    return;
  }

  const targetIndex = Math.min(area.chapters.length - 1, Math.max(0, currentIndex + offset));
  if (targetIndex !== currentIndex) {
    await manuscriptStore.moveChapter(chapter.id, area.id, targetIndex);
  }
}

async function moveChapterToAdjacentArea(
  area: AreaSnapshot,
  chapter: ChapterSnapshot,
  offset: number,
) {
  const currentAreaIndex = manuscriptStore.areas.findIndex((item) => item.id === area.id);
  const targetArea = manuscriptStore.areas[currentAreaIndex + offset];
  if (targetArea) {
    await manuscriptStore.moveChapter(chapter.id, targetArea.id, targetArea.chapters.length);
    expandArea(targetArea.id);
  }
}

async function moveSceneByOffset(chapter: ChapterSnapshot, scene: SceneSnapshot, offset: number) {
  const currentIndex = chapter.scenes.findIndex((item) => item.id === scene.id);
  if (currentIndex < 0) {
    return;
  }

  const targetIndex = Math.min(chapter.scenes.length - 1, Math.max(0, currentIndex + offset));
  if (targetIndex !== currentIndex) {
    await manuscriptStore.moveScene(scene.id, chapter.id, targetIndex);
  }
}

async function moveSceneToAdjacentChapter(
  chapter: ChapterSnapshot,
  scene: SceneSnapshot,
  offset: number,
) {
  const currentChapterIndex = manuscriptStore.chapters.findIndex((item) => item.id === chapter.id);
  const targetChapter = manuscriptStore.chapters[currentChapterIndex + offset];
  if (targetChapter) {
    await manuscriptStore.moveScene(scene.id, targetChapter.id, targetChapter.scenes.length);
    expandChapterPath(targetChapter.id);
  }
}

async function createSceneForActiveChapter() {
  await manuscriptStore.createScene(manuscriptStore.activeChapter?.id ?? "");
  expandActiveLocation();
}

async function createAreaFromNavigator() {
  await manuscriptStore.createArea();
  expandActiveLocation();
}

async function createChapterFromNavigator(areaId?: string) {
  await manuscriptStore.createChapter(areaId);
  expandActiveLocation();
}

async function createSceneFromNavigator(chapterId?: string) {
  await manuscriptStore.createScene(chapterId);
  expandActiveLocation();
}

async function deleteArea(area: AreaSnapshot) {
  const sceneCount = area.chapters.reduce((count, chapter) => count + chapter.scenes.length, 0);
  const message =
    area.chapters.length > 0
      ? t("dialogs.archiveArea", { name: area.title, chapters: area.chapters.length, scenes: sceneCount })
      : t("dialogs.archiveAreaEmpty", { name: area.title });

  if (window.confirm(message)) {
    await manuscriptStore.deleteArea(area.id);
  }
}

async function deleteChapter(chapter: ChapterSnapshot) {
  const message =
    chapter.scenes.length > 0
      ? t("dialogs.archiveChapter", { name: chapter.title, scenes: chapter.scenes.length })
      : t("dialogs.archiveChapterEmpty", { name: chapter.title });

  if (window.confirm(message)) {
    await manuscriptStore.deleteChapter(chapter.id);
  }
}

async function deleteScene(scene: SceneSnapshot) {
  if (window.confirm(t("dialogs.archiveScene", { name: scene.title }))) {
    await manuscriptStore.deleteScene(scene.id);
  }
}

function clearSceneTextSelection() {
  sceneTextSelection.start = 0;
  sceneTextSelection.end = 0;
  sceneTextSelection.text = "";
  sceneTextSelection.sceneId = "";
  aiStore.clearTextAlternatives();
  aiStore.clearTextImprovements();
}

function updateSceneTextSelection() {
  const textarea = sceneTextAreaRef.value;
  if (!textarea || !sceneDraft.id) {
    return;
  }

  const start = textarea.selectionStart ?? 0;
  const end = textarea.selectionEnd ?? 0;
  if (start === end) {
    return;
  }

  sceneTextSelection.start = start;
  sceneTextSelection.end = end;
  sceneTextSelection.text = sceneDraft.text.slice(start, end);
  sceneTextSelection.sceneId = sceneDraft.id;
  aiStore.clearTextAlternatives();
  aiStore.clearTextImprovements();
}

function handleSceneTextKeydown(event: KeyboardEvent) {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.isComposing) {
    return;
  }

  const key = event.key.toLowerCase();
  if (key === "z") {
    event.preventDefault();
    if (event.shiftKey) {
      void redoSceneText();
    } else {
      void undoSceneText();
    }
    return;
  }

  if (key === "y") {
    event.preventDefault();
    void redoSceneText();
  }
}

async function runSceneSummary() {
  if (!canRunSceneSummary.value) {
    return;
  }

  await persistSceneDraftNow();
  await aiStore.summarizeActiveScene(sceneDraft.id);
}

async function runSceneContradictions() {
  if (!canRunSceneContradictions.value) {
    return;
  }

  await persistSceneDraftNow();
  await aiStore.checkActiveSceneContradictions(sceneDraft.id);
}

async function runTextAlternatives() {
  if (!canRunTextAlternatives.value) {
    return;
  }

  const start = sceneTextSelection.start;
  const end = sceneTextSelection.end;
  await aiStore.generateTextAlternatives({
    sceneId: sceneDraft.id,
    selectedText: sceneTextSelection.text,
    contextBefore: sceneDraft.text.slice(Math.max(0, start - 700), start),
    contextAfter: sceneDraft.text.slice(end, end + 700),
  });
}

async function runTextImprovements() {
  if (!canRunTextImprovements.value) {
    return;
  }

  const mode = textImprovementMode.value;
  const start = mode === "selection" ? sceneTextSelection.start : 0;
  const end = mode === "selection" ? sceneTextSelection.end : sceneDraft.text.length;
  const text = mode === "selection" ? sceneTextSelection.text : sceneDraft.text;

  textImprovementTarget.mode = mode;
  textImprovementTarget.start = start;
  textImprovementTarget.end = end;
  textImprovementTarget.text = text;
  textImprovementTarget.sceneId = sceneDraft.id;

  await aiStore.improveText({
    sceneId: sceneDraft.id,
    selectedText: mode === "selection" ? sceneTextSelection.text : "",
    contextBefore: mode === "selection" ? sceneDraft.text.slice(Math.max(0, start - 900), start) : "",
    contextAfter: mode === "selection" ? sceneDraft.text.slice(end, end + 900) : "",
    mode,
  });
}

async function applyTextAlternative(alternative: string) {
  if (!sceneDraft.id || sceneTextSelection.sceneId !== sceneDraft.id) {
    window.alert(t("dialogs.selectionAgain"));
    return;
  }

  const { start, end, text } = sceneTextSelection;
  if (sceneDraft.text.slice(start, end) !== text) {
    window.alert(t("dialogs.selectionChanged"));
    clearSceneTextSelection();
    return;
  }

  sceneDraft.text = `${sceneDraft.text.slice(0, start)}${alternative}${sceneDraft.text.slice(end)}`;
  sceneTextSelection.end = start + alternative.length;
  sceneTextSelection.text = alternative;
  await nextTick();
  sceneTextAreaRef.value?.focus();
  sceneTextAreaRef.value?.setSelectionRange(start, sceneTextSelection.end);
  await persistSceneDraftNow();
}

async function applyTextImprovement(improvement: string) {
  if (!sceneDraft.id || textImprovementTarget.sceneId !== sceneDraft.id) {
    window.alert(t("dialogs.improvementAgain"));
    return;
  }

  if (textImprovementTarget.mode === "selection") {
    const { start, end, text } = textImprovementTarget;
    if (sceneDraft.text.slice(start, end) !== text) {
      window.alert(t("dialogs.improvementChanged"));
      aiStore.clearTextImprovements();
      return;
    }

    sceneDraft.text = `${sceneDraft.text.slice(0, start)}${improvement}${sceneDraft.text.slice(end)}`;
    sceneTextSelection.start = start;
    sceneTextSelection.end = start + improvement.length;
    sceneTextSelection.text = improvement;
    sceneTextSelection.sceneId = sceneDraft.id;
    textImprovementTarget.end = sceneTextSelection.end;
    textImprovementTarget.text = improvement;
    await nextTick();
    sceneTextAreaRef.value?.focus();
    sceneTextAreaRef.value?.setSelectionRange(start, sceneTextSelection.end);
    await persistSceneDraftNow();
    return;
  }

  if (!window.confirm(t("dialogs.acceptImprovement"))) {
    return;
  }

  sceneDraft.text = improvement;
  clearSceneTextSelection();
  await nextTick();
  sceneTextAreaRef.value?.focus();
  await persistSceneDraftNow();
}

async function acceptAiSummary() {
  if (activeSceneSummarySuggestion.value) {
    aiStore.activeSuggestionId = activeSceneSummarySuggestion.value.id;
  }

  const accepted = await aiStore.acceptActiveSuggestion();
  if (accepted) {
    await manuscriptStore.loadTree();
  }
}

async function rejectSceneAiSuggestion(id?: string) {
  if (id) {
    aiStore.activeSuggestionId = id;
  }

  await aiStore.rejectActiveSuggestion();
}

async function startLocalAiProvider() {
  await aiStore.startProvider();
}

async function selectAiModel(event: Event) {
  const modelName = (event.target as HTMLSelectElement).value;
  await aiStore.selectModel(modelName);
}

async function canOpenWorkspaceSection(section: WorkspaceSection) {
  if (workspace.activeSection === section) return true;
  return await workspaceNavigation.beforeLeave(workspace.activeSection);
}

async function openTopbarSection(section: WorkspaceSection) {
  if (!await canOpenWorkspaceSection(section)) return;
  workspace.setActiveSection(section);
  if (section === "structure" && workspace.sidebarTab !== "world") workspace.setSidebarTab("context");
}

async function openTopbarWriting() {
  if (!await canOpenWorkspaceSection("writing")) return;
  workspace.setActiveSection("writing");
  if (workspace.sidebarTab === "ai") {
    workspace.setSidebarTab("context");
  }
}

async function openTopbarBible() {
  if (!await canOpenWorkspaceSection("bible")) return;
  workspace.setActiveSection("bible");
}

async function openTopbarAi() {
  if (!await canOpenWorkspaceSection("ai")) return;
  workspace.setActiveSection("ai");
}

function searchResultKindLabel(kind: string, targetType?: KnowledgeTargetType | null) {
  if (kind === "scene") {
    return t("searchKinds.scene");
  }

  return targetType ? cardTypeSingular(targetType) : t("searchKinds.bible");
}

async function selectProjectSearchResult(result: SearchProjectResult) {
  if (result.kind === "scene" && result.sceneId) {
    await selectScene(result.sceneId);
    return;
  }

  if (result.kind === "bible" && result.targetType && result.targetId) {
    if (result.targetType === "world_system") { await openWorldSystem(result.targetId); return; }
    if (!await persistCardDraftNow()) return;
    projectBibleStore.setActiveCardType(result.targetType);
    projectBibleStore.selectCard(result.targetId);
    workspace.setActiveSection("bible");
  }
}

function updateSearchQuery(event: Event) {
  searchStore.setQuery((event.target as HTMLInputElement).value);
}

function defaultExportFileName(format: ExportFormat) {
  const extension = format === "markdown" ? "md" : "docx";
  const projectName = projectStore.activeProject?.name ?? "";
  const safeName = projectName
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "-")
    .replace(/\s+/g, " ");
  const manuscriptName = t("files.manuscript");

  return `${safeName ? `${safeName} - ` : ""}${manuscriptName}.${extension}`;
}

function safeProjectFileBaseName() {
  const projectName = projectStore.activeProject?.name || t("files.project");
  return (
    projectName
      .trim()
      .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "-")
      .replace(/\s+/g, " ") || t("files.project")
  );
}

function defaultProjectBackupFileName() {
  const now = new Date();
  const date = [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
  ].join("-");
  const time = `${String(now.getHours()).padStart(2, "0")}${String(now.getMinutes()).padStart(2, "0")}`;

  return `${safeProjectFileBaseName()} - ${t("files.snapshot")} ${date} ${time}.autorenprojekt`;
}

async function exportManuscript(format: ExportFormat) {
  if (exportStore.isExporting) {
    return;
  }

  exportStore.clear();

  try {
    const path = await chooseExportPath(format, defaultExportFileName(format));
    if (!path) {
      return;
    }

    await exportStore.exportManuscript(format, path);
  } catch (error) {
    exportStore.setError(error, "file_dialog_failed");
  }
}

async function saveProjectCopyAs() {
  if (projectStore.isSavingCopy) {
    return;
  }

  await persistSceneDraftNow();

  try {
    const path = await chooseProjectBackupPath(defaultProjectBackupFileName());
    if (!path) {
      return;
    }

    await projectStore.saveCopyAs(path);
  } catch (error) {
    projectStore.setCopyError(error, "file_dialog_failed");
  }
}

async function saveProjectSettings() {
  if (!canSaveProjectSettings.value) {
    return;
  }

  await projectStore.updateProject({
    name: projectSettingsDraft.name,
    description: projectSettingsDraft.description,
    defaultLanguage: projectSettingsDraft.defaultLanguage,
    editorFontFamily: projectSettingsDraft.editorFontFamily,
    editorFontSize: projectSettingsDraft.editorFontSize,
  });

  hydrateProjectSettings();
}

function relationshipCard(item: { sourceType: KnowledgeTargetType; sourceId: string; targetType: KnowledgeTargetType; targetId: string }, viewedType: KnowledgeTargetType = selectedCard.value?.targetType ?? "character", viewedId: string = selectedCard.value?.id ?? "") {
  const outgoing = item.sourceType === viewedType && item.sourceId === viewedId;
  return { outgoing, card: findProjectBibleCard(outgoing ? item.targetType : item.sourceType, outgoing ? item.targetId : item.sourceId) };
}

function relationshipLabel(item: Parameters<typeof relationshipDisplayLabel>[0], viewedType: KnowledgeTargetType = selectedCard.value?.targetType ?? "character", viewedId: string = selectedCard.value?.id ?? "") {
  return relationshipDisplayLabel(item, viewedType, viewedId, t);
}

async function addBibleRelationship() {
  const card = selectedCard.value;
  if (!card || !relationshipTargetId.value) return;
  const existing = projectBibleStore.snapshot?.relationships.find((item) => item.id === relationshipEditId.value);
  if (await projectBibleStore.saveRelationship({ id: relationshipEditId.value || undefined, sourceType: existing?.sourceType ?? card.targetType, sourceId: existing?.sourceId ?? card.id, targetType: existing?.targetType ?? relationshipTargetType.value, targetId: existing?.targetId ?? relationshipTargetId.value, relationshipType: relationshipType.value, description: relationshipDescription.value })) {
    relationshipEditId.value = ""; relationshipTargetId.value = ""; relationshipDescription.value = ""; relationshipSearch.value = "";
  }
}

function editBibleRelationship(item: typeof projectBibleStore.selectedCardRelationships[number]) {
  const related = relationshipCard(item);
  if (!related.card) return;
  relationshipEditId.value = item.id;
  relationshipTargetType.value = related.card.targetType;
  relationshipTargetId.value = related.card.id;
  relationshipType.value = item.relationshipType;
  relationshipDescription.value = item.description;
}

function editWorldRelationship(item: typeof projectBibleStore.selectedCardRelationships[number]) {
  const related = relationshipCard(item, "world_system", worldbuildingStore.selectedSystemId);
  if (!related.card) return;
  relationshipEditId.value = item.id;
  relationshipTargetType.value = related.card.targetType;
  relationshipTargetId.value = related.card.id;
  relationshipType.value = item.relationshipType;
  relationshipDescription.value = item.description;
}

async function moveSelectedEvent(direction: -1 | 1) {
  if (projectBibleStore.activeCardType !== "event") return;
  const ids = projectBibleStore.cards.map((card) => card.id);
  const index = ids.indexOf(projectBibleStore.selectedCardId);
  const target = index + direction;
  if (index < 0 || target < 0 || target >= ids.length) return;
  [ids[index], ids[target]] = [ids[target], ids[index]];
  await projectBibleStore.reorderEvents(ids);
}

async function dropBibleEvent(targetId: string) {
  const sourceId = draggedEventId.value; draggedEventId.value = "";
  if (!sourceId || sourceId === targetId) return;
  const ids = projectBibleStore.cards.map((card) => card.id);
  const from = ids.indexOf(sourceId); const to = ids.indexOf(targetId);
  if (from < 0 || to < 0) return;
  ids.splice(to, 0, ids.splice(from, 1)[0]);
  await projectBibleStore.reorderEvents(ids);
}

function updateExportOption(key: keyof ExportProfile, event: Event) {
  void exportStore.setProfileOption(key, (event.target as HTMLInputElement).checked);
}

async function saveProjectDefaults() {
  await projectStore.saveProjectDefaults(projectDefaultsTemplateId.value);
}

async function sendAiChat() {
  const text = aiChatDraft.value.trim();
  if (!text || aiStore.isRunningChat) {
    return;
  }

  aiChatDraft.value = "";
  await aiStore.sendChatMessage(
    text,
    manuscriptStore.activeSceneId,
    aiChatIncludeSceneContext.value,
    aiChatMode.value,
  );
}

function handleAiChatKeydown(event: KeyboardEvent) {
  if (event.key !== "Enter" || event.shiftKey || event.isComposing) {
    return;
  }

  event.preventDefault();
  void sendAiChat();
}

function cleanAiChatTextForScene(content: string) {
  let text = content.replace(/\r\n/g, "\n").trim();
  const sectionMarker = /(?:^|\n)\s*(?:Erweiterte Szene|Ueberarbeitete Szene|Überarbeitete Szene|Neue Fassung|Szenentext|Expanded Scene|Revised Scene|New Version|Scene Text|Text)\s*:\s*/gi;
  const matches = [...text.matchAll(sectionMarker)];
  if (matches.length) {
    const lastMatch = matches[matches.length - 1];
    text = text.slice((lastMatch.index ?? 0) + lastMatch[0].length).trim();
  }

  text = text
    .replace(/^```(?:\w+)?\s*/i, "")
    .replace(/\s*```$/i, "")
    .trim();

  const lines = text.split("\n");
  while (
    lines.length &&
    (/^\s*$/.test(lines[0]) ||
      /^(Aktuelle Szene|Titel|Status|Kurzbeschreibung|Szenentext-Auszug|Projektbibel-Kontext|KI|Antwort|Current Scene|Title|Summary|Scene Text Excerpt|Story Bible Context|AI|Response)\s*:/i.test(
        lines[0].trim(),
      ) ||
      /^(?:Hier ist|Here is)\b.*:\s*$/i.test(lines[0].trim()))
  ) {
    lines.shift();
  }

  return lines.join("\n").trim();
}

function canAcceptAiChatText(content: string) {
  return Boolean(sceneDraft.id && cleanAiChatTextForScene(content));
}

async function acceptAiChatMessage(content: string) {
  const cleanedText = cleanAiChatTextForScene(content);
  if (!sceneDraft.id || !cleanedText) {
    return;
  }

  if (!window.confirm(t("dialogs.acceptChat"))) {
    return;
  }

  sceneDraft.text = cleanedText;
  await nextTick();
  await persistSceneDraftNow();
}

async function runBibleAi() {
  if (!canRunBibleAi.value) {
    return;
  }

  if (!await flushCardDraftSave()) return;
  if (bibleAiMode.value === "generate") {
    await aiStore.generateBibleCard(cardDraft.targetType, bibleAiKeywords.value);
  } else {
    await aiStore.editBibleCard(cardDraft.targetType, cardDraft.id, bibleAiKeywords.value);
  }

  hydrateBibleAiDraft();
}

async function acceptBibleAiSuggestion() {
  const result = await aiStore.acceptActiveBibleSuggestion(
    { ...bibleAiDraft },
    bibleAiSelectedFields.value,
  );
  if (!result) {
    return;
  }

  projectBibleStore.setActiveCardType(result.targetType);
  projectBibleStore.setSnapshot(result.snapshot, result.targetId);
  hydrateBibleAiDraft();
}

async function rejectBibleAiSuggestion() {
  await aiStore.rejectActiveBibleSuggestion();
}

onMounted(loadStoredLayout);

watch(
  () => workspace.layout,
  (layout) => {
    window.localStorage.setItem(layoutStorageKey, JSON.stringify(layout));
  },
  { deep: true },
);

watch(
  () => projectStore.activeProject?.path,
  async (path) => {
    sceneAutosave.reset();
    cardAutosave.reset();
    worldSystemAutosave.reset();
    beatAutosave.reset();
    window.clearTimeout(searchTimer);
    clearAllSceneTextHistories();
    loadStoredNavigatorCollapse(path);
    searchStore.reset();

    if (path) {
      await storyStructureStore.load();
      await Promise.all([
        manuscriptStore.loadTree(),
        projectBibleStore.load(),
        worldbuildingStore.load(),
        exportStore.loadProfile(),
      ]);
      pruneNavigatorCollapse();
      await aiStore.loadSceneSuggestions(manuscriptStore.activeSceneId);
      return;
    }

    manuscriptStore.reset();
    projectBibleStore.reset();
    worldbuildingStore.$reset();
    aiStore.reset();
    storyStructureStore.reset();
    exportStore.reset();
  },
  { immediate: true },
);

watch(
  () => projectStore.activeProject,
  (project) => { setLocale(projectLanguage(project?.default_language)); hydrateProjectSettings(); },
  { immediate: true },
);

watch(
  () => projectStore.projectDefaults?.storyStructureTemplateId,
  (templateId) => {
    projectDefaultsTemplateId.value = templateId ?? "freestyle";
  },
  { immediate: true },
);

watch(
  () => manuscriptStore.tree,
  () => pruneNavigatorCollapse(),
  { deep: true },
);

watch(
  () => manuscriptStore.activeSceneId,
  async (sceneId) => {
    clearSceneTextSelection();
    await aiStore.loadSceneSuggestions(sceneId);
  },
);

watch(
  () => `${workspace.activeSection}:${workspace.sidebarTab}`,
  async (activePanel) => {
    if (
      activePanel.startsWith("ai:") ||
      activePanel.endsWith(":ai") ||
      activePanel.startsWith("settings:")
    ) {
      await aiStore.checkProviderStatus();
      await aiStore.loadModels();
    }

    if (activePanel === "bible:ai") {
      await aiStore.loadBibleSuggestions(
        projectBibleStore.activeCardType,
        projectBibleStore.selectedCardId,
      );
      hydrateBibleAiDraft();
    }
  },
);

watch(
  () => manuscriptStore.activeScene,
  (scene) => hydrateSceneDraft(scene),
  { immediate: true },
);

watch(
  () => `${manuscriptStore.activeSceneId}:${firstWritingBibleCardKey.value}`,
  async () => {
    const cards = writingBibleGroups.value.flatMap((group) => group.cards);
    const openCard = cards.find((card) => projectBibleCardKey(card) === writingBibleOpenCardKey.value);
    const nextCard = openCard ?? cards[0] ?? null;

    if (nextCard && selectedCardKey.value !== projectBibleCardKey(nextCard)) {
      if (!await persistCardDraftNow()) return;
    }

    writingBibleOpenCardKey.value = nextCard ? projectBibleCardKey(nextCard) : "";
    if (nextCard && selectedCardKey.value !== projectBibleCardKey(nextCard)) {
      projectBibleStore.setActiveCardType(nextCard.targetType);
      projectBibleStore.selectCard(nextCard.id);
    }
  },
  { immediate: true },
);

watch(
  sceneDraft,
  () => scheduleSceneSave(),
  { deep: true },
);

watch(
  () => sceneDraft.text,
  (newText, oldText) => {
    if (isHydratingSceneDraft.value || isApplyingTextHistory.value || !sceneDraft.id) {
      return;
    }

    recordSceneTextHistoryChange(sceneDraft.id, oldText ?? "", newText);
  },
);

watch(
  selectedCardKey,
  async () => {
    relationshipEditId.value = "";
    relationshipTargetId.value = "";
    relationshipDescription.value = "";
    hydrateCardDraft(projectBibleStore.selectedCard);
    if (workspace.activeSection === "bible" && workspace.sidebarTab === "ai") {
      await aiStore.loadBibleSuggestions(
        projectBibleStore.activeCardType,
        projectBibleStore.selectedCardId,
      );
      hydrateBibleAiDraft();
    }
  },
  { immediate: true },
);

watch(() => worldbuildingStore.selectedSystemId, () => {
  relationshipEditId.value = "";
  relationshipTargetId.value = "";
  relationshipDescription.value = "";
  hydrateWorldSystemDraft();
});
watch(worldSystemDraft, () => scheduleWorldSystemSave(), { deep: true });
watch(() => storyStructureStore.selectedBeatId, () => hydrateBeatDraft(), { immediate: true });
watch(beatDraft, () => scheduleBeatSave(), { deep: true });
watch(newWorldSystemCategory, (category) => { newWorldSystemTemplateId.value = defaultTemplateForCategory(category); });

watch(relationshipTargetType, () => { if (!relationshipEditId.value) relationshipTargetId.value = ""; });
watch(
  () => `${relationshipEndpoints.value.sourceType}:${relationshipEndpoints.value.targetType}:${relationshipEditId.value}`,
  () => {
    if (!availableRelationshipTypes.value.some((type) => type.id === relationshipType.value)) {
      relationshipType.value = "related";
    }
  },
);

watch(
  () => aiStore.activeBibleSuggestion?.id,
  () => hydrateBibleAiDraft(),
);

watch(
  cardDraft,
  () => scheduleCardSave(),
  { deep: true },
);

watch(
  () => [searchStore.query, searchStore.scope],
  () => {
    window.clearTimeout(searchTimer);
    searchTimer = window.setTimeout(() => {
      void searchStore.runSearch();
    }, 250);
  },
);


  return {
    computed,
    nextTick,
    onMounted,
    reactive,
    ref,
    watch,
    useI18n,
    chooseExportPath,
    chooseProjectBackupPath,
    localeCode,
    projectLanguage,
    setLocale,
    formatUiError,
    generatedUi,
    createDraftAutosave,
    isRelationshipTypeAllowed,
    relationshipDefinitions,
    relationshipDisplayLabel,
    useAiStore,
    useExportStore,
    useManuscriptStore,
    useProjectStore,
    useSearchStore,
    useStoryStructureStore,
    useWorldbuildingStore,
    worldSystemCategoryDefinitions,
    noteTypeDefinitions,
    projectBibleCardTypeDefinitions,
    useProjectBibleStore,
    defaultWorkspaceLayout,
    useWorkspaceStore,
    workspace,
    locale,
    t,
    ui,
    projectBibleCardTypes,
    noteTypes,
    worldSystemCategories,
    projectStore,
    manuscriptStore,
    projectBibleStore,
    aiStore,
    searchStore,
    exportStore,
    storyStructureStore,
    worldbuildingStore,
    pointerDrag,
    pointerDropKey,
    focusedNavigatorKey,
    collapsedAreaIds,
    collapsedChapterIds,
    isHydratingSceneDraft,
    isHydratingCardDraft,
    isHydratingWorldSystemDraft,
    isApplyingTextHistory,
    layoutStorageKey,
    navigatorCollapseStorageKeyPrefix,
    sceneTextHistoryLimit,
    searchTimer,
    isHydratingBeatDraft,
    writingBibleOpenCardKey,
    writingBibleAddType,
    writingBibleSearchQuery,
    relationshipTargetType,
    relationshipTargetId,
    relationshipType,
    relationshipDescription,
    relationshipEditId,
    relationshipSearch,
    draggedEventId,
    showWorldSystemDialog,
    newWorldSystemName,
    newWorldSystemCategory,
    newWorldSystemTemplateId,
    worldSystemDraft,
    beatDraft,
    worldSystemFieldLabels,
    worldSystemOptionalFields,
    sceneStatuses,
    sceneStatusLabel,
    editorFontOptions,
    editorFontSizeOptions,
    editorFontStacks,
    sceneDraft,
    cardDraft,
    sceneAutosave,
    cardAutosave,
    worldSystemAutosave,
    beatAutosave,
    worldSystemSavePending,
    projectSettingsDraft,
    projectDefaultsTemplateId,
    aiChatDraft,
    aiChatIncludeSceneContext,
    aiChatMode,
    sidebarAiAction,
    sceneTextAreaRef,
    sceneTextSelection,
    sceneTextHistories,
    textImprovementTarget,
    bibleAiMode,
    bibleAiKeywords,
    bibleAiSelectedFields,
    bibleAiDraft,
    panelBounds,
    sidebarTabs,
    sidebarTitle,
    structureCategoryLabels,
    structureBeatCount,
    activeSceneStructureAssignments,
    structureCharacters,
    structureAreaTitle,
    structureChapterTitle,
    hydrateBeatDraft,
    scheduleBeatSave,
    flushBeatSave,
    selectStructureBeat,
    toggleBeatScene,
    toggleBeatCharacter,
    toggleActiveSceneBeat,
    structureIsPreview,
    structurePositionLabel,
    applySelectedStructureTemplate,
    selectStructureTemplate,
    generateActiveStructureManuscript,
    workspaceStyle,
    sceneEditorStyle,
    saveState,
    projectCopyState,
    bibleSaveState,
    worldbuildingSaveState,
    selectedCard,
    selectedCardKey,
    selectedCardSceneIds,
    selectedCardTypeLabel,
    selectedWorldTemplate,
    worldSystemNameMissing,
    openWorldSystemFields,
    sceneWorldSystems,
    selectedCardWorldSystems,
    worldOverviewSystems,
    relationshipTypes,
    relationshipEndpoints,
    availableRelationshipTypes,
    relationshipCandidates,
    currentSceneTextHistory,
    canUndoSceneText,
    canRedoSceneText,
    writingBibleSceneLinks,
    writingBibleGroups,
    writingBibleLinkedCardKeys,
    writingBibleAvailableCards,
    firstWritingBibleCardKey,
    aiContextPreview,
    canRunSceneSummary,
    canRunSceneContradictions,
    sceneSuggestionForAction,
    activeSceneSummarySuggestion,
    activeSceneContradictionSuggestion,
    selectedSceneText,
    selectedSceneTextLabel,
    canRunTextAlternatives,
    textImprovementMode,
    canRunTextImprovements,
    canRunBibleAi,
    bibleAiHasDraft,
    bibleAiChangedFields,
    canSaveProjectSettings,
    bibleAiDuplicateName,
    aiProviderStateLabel,
    selectedAiModelInfo,
    aiConfiguredModelLabel,
    aiModelMissing,
    aiModelListEmpty,
    clampPanelWidth,
    loadStoredLayout,
    navigatorCollapseStorageKey,
    loadStoredNavigatorCollapse,
    saveStoredNavigatorCollapse,
    pruneNavigatorCollapse,
    isAreaCollapsed,
    isChapterCollapsed,
    toggleAreaCollapsed,
    toggleChapterCollapsed,
    expandArea,
    expandChapter,
    expandChapterPath,
    expandActiveLocation,
    startPanelResize,
    ensureSceneTextHistory,
    clearAllSceneTextHistories,
    recordSceneTextHistoryChange,
    applySceneTextFromHistory,
    undoSceneText,
    redoSceneText,
    hydrateSceneDraft,
    hydrateCardDraft,
    hydrateProjectSettings,
    normalizeEditorFontFamily,
    normalizeEditorFontSize,
    formatProjectTimestamp,
    hydrateBibleAiDraft,
    hydrateWorldSystemDraft,
    scheduleWorldSystemSave,
    flushWorldSystemDraftSave,
    persistWorldSystemDraftNow,
    selectWorldSystem,
    defaultTemplateForCategory,
    worldCategoryLabel,
    openNewWorldSystemDialog,
    createWorldSystemFromDialog,
    openWorldSystem,
    archiveWorldSystem,
    toggleSceneWorldSystem,
    linkSelectedCardToWorld,
    addWorldRelationship,
    scheduleSceneSave,
    persistSceneDraftNow,
    scheduleCardSave,
    currentCardDraftInput,
    flushCardDraftSave,
    persistCardDraftNow,
    cardTypeLabel,
    cardTypeSingular,
    noteTypeLabel,
    modelOptionLabel,
    formatModelSize,
    projectBibleCardKey,
    isWritingBibleCardOpen,
    toggleWritingBibleCard,
    projectBibleCardsForType,
    findProjectBibleCard,
    bibleCardNavigatorKey,
    selectBibleCard,
    createBibleCard,
    selectBibleCardType,
    createWritingBibleCard,
    addWritingBibleCard,
    archiveSelectedBibleCard,
    isSceneLinkedToSelectedCard,
    toggleSelectedCardScene,
    selectScene,
    selectChapter,
    selectArea,
    updateAreaTitle,
    updateChapterTitle,
    commitChapterTitle,
    areaNavigatorKey,
    chapterNavigatorKey,
    sceneNavigatorKey,
    focusNavigatorItem,
    blurNavigatorItem,
    restoreNavigatorFocus,
    handleAreaKeydown,
    handleChapterKeydownInArea,
    handleSceneKeydown,
    areaDropKey,
    areaEndDropKey,
    chapterDropKey,
    sceneDropKey,
    chapterEndDropKey,
    readDropTarget,
    targetDropKey,
    startPointerDrag,
    movePointerDrag,
    cancelPointerDrag,
    finishPointerDrag,
    applyPointerDrop,
    startAreaPointerDrag,
    startChapterPointerDrag,
    startScenePointerDrag,
    moveAreaByOffset,
    moveChapterByOffset,
    moveChapterToAdjacentArea,
    moveSceneByOffset,
    moveSceneToAdjacentChapter,
    createSceneForActiveChapter,
    createAreaFromNavigator,
    createChapterFromNavigator,
    createSceneFromNavigator,
    deleteArea,
    deleteChapter,
    deleteScene,
    clearSceneTextSelection,
    updateSceneTextSelection,
    handleSceneTextKeydown,
    runSceneSummary,
    runSceneContradictions,
    runTextAlternatives,
    runTextImprovements,
    applyTextAlternative,
    applyTextImprovement,
    acceptAiSummary,
    rejectSceneAiSuggestion,
    startLocalAiProvider,
    selectAiModel,
    canOpenWorkspaceSection,
    openTopbarSection,
    openTopbarWriting,
    openTopbarBible,
    openTopbarAi,
    searchResultKindLabel,
    selectProjectSearchResult,
    updateSearchQuery,
    defaultExportFileName,
    safeProjectFileBaseName,
    defaultProjectBackupFileName,
    exportManuscript,
    saveProjectCopyAs,
    saveProjectSettings,
    relationshipCard,
    relationshipLabel,
    addBibleRelationship,
    editBibleRelationship,
    editWorldRelationship,
    moveSelectedEvent,
    dropBibleEvent,
    updateExportOption,
    saveProjectDefaults,
    sendAiChat,
    handleAiChatKeydown,
    cleanAiChatTextForScene,
    canAcceptAiChatText,
    acceptAiChatMessage,
    runBibleAi,
    acceptBibleAiSuggestion,
    rejectBibleAiSuggestion,
  };
}
