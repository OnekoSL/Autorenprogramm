<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";
import { getAppInfo, type AppInfo } from "./api/app";
import type { AiBibleCardDraft, AiChatMode, ImproveTextMode } from "./api/ai";
import { chooseExportPath, type ExportFormat } from "./api/export";
import type { AreaSnapshot, ChapterSnapshot, SceneSnapshot } from "./api/manuscript";
import { chooseProjectBackupPath } from "./api/project";
import type { NoteType, ProjectBibleCardType, ProjectBibleCardUpdate } from "./api/projectBible";
import type { SearchProjectResult } from "./api/search";
import ProjectStartView from "./components/ProjectStartView.vue";
import { useAiStore } from "./stores/ai";
import { useExportStore } from "./stores/export";
import { useManuscriptStore } from "./stores/manuscript";
import { useProjectStore } from "./stores/project";
import { useSearchStore } from "./stores/search";
import {
  noteTypes,
  projectBibleCardTypes,
  useProjectBibleStore,
  type ProjectBibleCard,
} from "./stores/projectBible";
import {
  defaultWorkspaceLayout,
  useWorkspaceStore,
  type LayoutPanel,
  type SidebarTab,
  type WorkspaceLayout,
  type WorkspaceSection,
} from "./stores/workspace";

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
const projectStore = useProjectStore();
const manuscriptStore = useManuscriptStore();
const projectBibleStore = useProjectBibleStore();
const aiStore = useAiStore();
const searchStore = useSearchStore();
const exportStore = useExportStore();
const appInfo = ref<AppInfo | null>(null);
const appInfoFailed = ref(false);
const pointerDrag = ref<PointerDragState | null>(null);
const pointerDropKey = ref("");
const focusedNavigatorKey = ref("");
const collapsedAreaIds = ref(new Set<string>());
const collapsedChapterIds = ref(new Set<string>());
const isHydratingSceneDraft = ref(false);
const isHydratingCardDraft = ref(false);
const isApplyingTextHistory = ref(false);
const layoutStorageKey = "autorenprogramm.workspace.layout";
const navigatorCollapseStorageKeyPrefix = "autorenprogramm.navigator.collapse";
const sceneTextHistoryLimit = 50;
let saveTimer: number | undefined;
let cardSaveTimer: number | undefined;
let searchTimer: number | undefined;
let cardDraftRevision = 0;
let cardSaveInFlight = false;
const writingBibleOpenCardKey = ref("");
const writingBibleAddType = ref<ProjectBibleCardType>("character");
const writingBibleSearchQuery = ref("");

const sceneStatuses = ["Entwurf", "Ueberarbeitung", "Fertig"];
type EditorFontFamily = "serif" | "sans" | "monospace";
type BibleAiMode = "generate" | "complete";
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
const sceneDraft = reactive({
  id: "",
  title: "",
  status: "Entwurf",
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
const projectSettingsDraft = reactive({
  name: "",
  description: "",
  defaultLanguage: "de",
  editorFontFamily: "serif" as EditorFontFamily,
  editorFontSize: 19,
});
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
  if (workspace.activeSection === "bible") {
    return [
      { id: "context", label: "Szenen" },
      { id: "bible", label: "Bibel" },
      { id: "ai", label: "KI" },
    ];
  }

  return [
    { id: "context", label: "Kontext" },
    { id: "bible", label: "Bibel" },
    { id: "ai", label: "KI" },
  ];
});

const sidebarTitle = computed(() => {
  if (workspace.activeSection === "bible") {
    if (workspace.sidebarTab === "context") {
      return "Szenenzuordnung";
    }

    if (workspace.sidebarTab === "ai") {
      return "Karten-KI";
    }

    return "Kartenuebersicht";
  }

  if (workspace.sidebarTab === "ai") {
    return "KI-Seitenleiste";
  }

  if (workspace.sidebarTab === "bible") {
    return "Projektbibel";
  }

  return "Szenenkontext";
});

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
    return "Fehler";
  }

  if (manuscriptStore.isSaving) {
    return "Speichert";
  }

  return "Gespeichert";
});

const projectCopyState = computed(() => {
  if (projectStore.copyError) {
    return "Backup-Fehler";
  }

  if (projectStore.isSavingCopy) {
    return "Sichert Backup";
  }

  if (projectStore.lastCopyPath) {
    return "Backup erstellt";
  }

  return "";
});

const bibleSaveState = computed(() => {
  if (projectBibleStore.error) {
    return "Fehler";
  }

  if (projectBibleStore.isSaving) {
    return "Speichert";
  }

  return "Gespeichert";
});

const selectedCard = computed(() => projectBibleStore.selectedCard);
const selectedCardKey = computed(
  () => `${projectBibleStore.activeCardType}:${projectBibleStore.selectedCardId}`,
);
const selectedCardSceneIds = computed(
  () => new Set(projectBibleStore.selectedCardLinks.map((link) => link.sceneId)),
);

const selectedCardTypeLabel = computed(() => cardTypeLabel(projectBibleStore.activeCardType));

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
  };

  for (const link of writingBibleSceneLinks.value) {
    const card = findProjectBibleCard(link.targetType, link.targetId);
    if (card && !groupedCards[card.targetType].some((item) => item.id === card.id)) {
      groupedCards[card.targetType].push(card);
    }
  }

  return projectBibleCardTypes
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
  const query = writingBibleSearchQuery.value.trim().toLocaleLowerCase();

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
      .toLocaleLowerCase()
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
    sceneDraft.title || "Aktuelle Szene",
    manuscriptStore.activeLocation,
    sceneDraft.summary || "Keine Kurzbeschreibung",
    `${linkedCards} Kontextkarte(n) verknuepft`,
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

const bibleAiEmptyFields = computed(() => {
  if (!selectedCard.value) {
    return [];
  }

  if (cardDraft.targetType === "character") {
    return [
      isBlankOrDefault(cardDraft.name, "Neue Figur") ? "Name" : "",
      cardDraft.role.trim() ? "" : "Rolle",
      cardDraft.description.trim() ? "" : "Beschreibung",
      cardDraft.notes.trim() ? "" : "Notizen",
    ].filter(Boolean);
  }

  if (cardDraft.targetType === "location") {
    return [
      isBlankOrDefault(cardDraft.name, "Neuer Ort") ? "Name" : "",
      cardDraft.description.trim() ? "" : "Beschreibung",
      cardDraft.notes.trim() ? "" : "Notizen",
    ].filter(Boolean);
  }

  if (cardDraft.targetType === "item") {
    return [
      isBlankOrDefault(cardDraft.name, "Neuer Gegenstand") ? "Name" : "",
      cardDraft.description.trim() ? "" : "Beschreibung",
      cardDraft.notes.trim() ? "" : "Notizen",
    ].filter(Boolean);
  }

  return [
    isBlankOrDefault(cardDraft.title, "Neue Notiz") ? "Titel" : "",
    cardDraft.body.trim() ? "" : "Inhalt",
  ].filter(Boolean);
});

const canRunBibleAi = computed(
  () =>
    Boolean(bibleAiKeywords.value.trim()) &&
    aiStore.isProviderReachable &&
    aiStore.isConfiguredModelAvailable &&
    !aiStore.isRunningBible &&
    (bibleAiMode.value === "generate" || Boolean(selectedCard.value?.id)),
);

const bibleAiHasDraft = computed(() => {
  if (cardDraft.targetType === "note") {
    return Boolean(bibleAiDraft.title.trim() || bibleAiDraft.body.trim());
  }

  return Boolean(
    bibleAiDraft.name.trim() ||
      bibleAiDraft.role.trim() ||
      bibleAiDraft.description.trim() ||
      bibleAiDraft.notes.trim(),
  );
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
  const normalized = title.trim().toLocaleLowerCase();
  if (!normalized) {
    return false;
  }

  return projectBibleStore.cards.some((card) => card.title.trim().toLocaleLowerCase() === normalized);
});

const aiProviderStateLabel = computed(() => {
  if (aiStore.isCheckingProvider) {
    return "Prueft lokale KI";
  }

  return aiStore.providerStatus?.message ?? "KI-Status noch nicht geprueft.";
});

const selectedAiModelInfo = computed(() => {
  const modelName = aiStore.settings?.model ?? "";

  return aiStore.models.find((model) => model.name === modelName || model.id === modelName) ?? null;
});

const aiConfiguredModelLabel = computed(() => {
  if (selectedAiModelInfo.value) {
    return modelOptionLabel(selectedAiModelInfo.value);
  }

  return aiStore.settings?.model || "Kein Modell ausgewaehlt";
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
  isHydratingSceneDraft.value = true;
  sceneDraft.id = scene?.id ?? "";
  sceneDraft.title = scene?.title ?? "";
  sceneDraft.status = scene?.status ?? "Entwurf";
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
  cardDraftRevision = 0;
  window.clearTimeout(cardSaveTimer);
  cardDraft.id = card?.id ?? "";
  cardDraft.targetType = card?.targetType ?? projectBibleStore.activeCardType;
  cardDraft.name = card && "name" in card ? card.name : "";
  cardDraft.role = card && "role" in card ? card.role : "";
  cardDraft.description = card && "description" in card ? card.description : "";
  cardDraft.notes = card && "notes" in card ? card.notes : "";
  cardDraft.title = card && "title" in card ? card.title : "";
  cardDraft.body = card && "body" in card ? card.body : "";
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

  return new Intl.DateTimeFormat("de-DE", {
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
}

function scheduleSceneSave() {
  if (isHydratingSceneDraft.value || isApplyingTextHistory.value || !sceneDraft.id) {
    return;
  }

  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    void manuscriptStore.updateScene({
      id: sceneDraft.id,
      title: sceneDraft.title,
      status: sceneDraft.status,
      summary: sceneDraft.summary,
      text: sceneDraft.text,
    });
  }, 400);
}

async function persistSceneDraftNow() {
  if (isHydratingSceneDraft.value || !sceneDraft.id) {
    return;
  }

  window.clearTimeout(saveTimer);
  await manuscriptStore.updateScene({
    id: sceneDraft.id,
    title: sceneDraft.title,
    status: sceneDraft.status,
    summary: sceneDraft.summary,
    text: sceneDraft.text,
  });
}

function scheduleCardSave() {
  if (isHydratingCardDraft.value || !cardDraft.id) {
    return;
  }

  cardDraftRevision += 1;
  window.clearTimeout(cardSaveTimer);
  cardSaveTimer = window.setTimeout(() => {
    void flushCardDraftSave();
  }, 400);
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

  return {
    targetType: cardDraft.targetType,
    id: cardDraft.id,
    name: cardDraft.name,
    role: cardDraft.role,
    description: cardDraft.description,
    notes: cardDraft.notes,
  };
}

async function flushCardDraftSave() {
  if (cardSaveInFlight || !cardDraft.id) {
    return;
  }

  cardSaveInFlight = true;

  try {
    while (cardDraft.id) {
      const revision = cardDraftRevision;
      const input = currentCardDraftInput();
      await projectBibleStore.updateCard(input);

      if (cardDraft.id !== input.id || cardDraft.targetType !== input.targetType) {
        break;
      }

      if (cardDraftRevision === revision) {
        break;
      }
    }
  } finally {
    cardSaveInFlight = false;
  }
}

async function persistCardDraftNow() {
  window.clearTimeout(cardSaveTimer);
  await flushCardDraftSave();
}

function cardTypeLabel(type: string) {
  return projectBibleCardTypes.find((cardType) => cardType.id === type)?.label ?? "Karten";
}

function cardTypeSingular(type: string) {
  if (type === "character") {
    return "Figur";
  }

  if (type === "location") {
    return "Ort";
  }

  if (type === "item") {
    return "Gegenstand";
  }

  return "Notiz";
}

function noteTypeLabel(type: string) {
  return noteTypes.find((noteType) => noteType.id === type)?.label ?? type;
}

function modelOptionLabel(model: { name: string; size: number }) {
  if (!model.size) {
    return model.name;
  }

  return `${model.name} (${formatModelSize(model.size)})`;
}

function isBlankOrDefault(value: string, defaultValue: string) {
  const trimmed = value.trim();
  return !trimmed || trimmed === defaultValue;
}

function formatModelSize(size: number) {
  if (size >= 1024 * 1024 * 1024) {
    return `${(size / 1024 / 1024 / 1024).toFixed(1)} GB`;
  }

  if (size >= 1024 * 1024) {
    return `${Math.round(size / 1024 / 1024)} MB`;
  }

  return `${size} B`;
}

function projectBibleCardKey(card: Pick<ProjectBibleCard, "targetType" | "id">) {
  return `${card.targetType}:${card.id}`;
}

function isWritingBibleCardOpen(card: ProjectBibleCard) {
  return writingBibleOpenCardKey.value === projectBibleCardKey(card);
}

async function toggleWritingBibleCard(card: ProjectBibleCard) {
  const key = projectBibleCardKey(card);
  if (writingBibleOpenCardKey.value === key) {
    await persistCardDraftNow();
    writingBibleOpenCardKey.value = "";
    return;
  }

  await persistCardDraftNow();
  projectBibleStore.setActiveCardType(card.targetType);
  projectBibleStore.selectCard(card.id);
  writingBibleOpenCardKey.value = key;
}

function projectBibleCardsForType(targetType: ProjectBibleCardType): ProjectBibleCard[] {
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

  return snapshot.notes.map((card) => ({
    ...card,
    targetType: "note",
    name: card.title,
    subtitle: noteTypeLabel(card.noteType),
  }));
}

function findProjectBibleCard(
  targetType: ProjectBibleCardType,
  targetId: string,
): ProjectBibleCard | null {
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

function selectBibleCard(card: ProjectBibleCard) {
  projectBibleStore.selectCard(card.id);
  focusNavigatorItem(bibleCardNavigatorKey(card.id));
}

async function createBibleCard() {
  await projectBibleStore.createCard();
}

async function createWritingBibleCard() {
  if (!manuscriptStore.activeSceneId) {
    return;
  }

  await persistCardDraftNow();
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

  await persistCardDraftNow();
  await projectBibleStore.addSceneLinkToCard(card.targetType, card.id, manuscriptStore.activeSceneId);
  writingBibleOpenCardKey.value = projectBibleCardKey(card);
}

async function archiveSelectedBibleCard() {
  const card = selectedCard.value;
  if (!card) {
    return;
  }

  if (window.confirm(`${cardTypeSingular(card.targetType)} "${card.title}" archivieren?`)) {
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

function selectChapter(chapter: ChapterSnapshot) {
  const firstScene = chapter.scenes[0];
  if (firstScene) {
    manuscriptStore.selectScene(firstScene.id);
  }
}

function selectArea(area: AreaSnapshot) {
  const firstScene = area.chapters.find((chapter) => chapter.scenes.length > 0)?.scenes[0];
  if (firstScene) {
    manuscriptStore.selectScene(firstScene.id);
  }
}

async function updateAreaTitle(area: AreaSnapshot, event: Event) {
  const input = event.target as HTMLInputElement;
  const title = input.value.trim() || "Neuer Bereich";

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
  const title = input.value.trim() || "Neues Kapitel";

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
      ? `Bereich "${area.title}" mit ${area.chapters.length} Kapitel(n) und ${sceneCount} Szene(n) archivieren?`
      : `Bereich "${area.title}" archivieren?`;

  if (window.confirm(message)) {
    await manuscriptStore.deleteArea(area.id);
  }
}

async function deleteChapter(chapter: ChapterSnapshot) {
  const message =
    chapter.scenes.length > 0
      ? `Kapitel "${chapter.title}" und ${chapter.scenes.length} Szene(n) archivieren?`
      : `Kapitel "${chapter.title}" archivieren?`;

  if (window.confirm(message)) {
    await manuscriptStore.deleteChapter(chapter.id);
  }
}

async function deleteScene(scene: SceneSnapshot) {
  if (window.confirm(`Szene "${scene.title}" archivieren?`)) {
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
    window.alert("Bitte markiere den Text erneut.");
    return;
  }

  const { start, end, text } = sceneTextSelection;
  if (sceneDraft.text.slice(start, end) !== text) {
    window.alert("Der Szenentext hat sich seit der Markierung geaendert. Bitte markiere den Text erneut.");
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
    window.alert("Bitte fuehre die Textverbesserung erneut aus.");
    return;
  }

  if (textImprovementTarget.mode === "selection") {
    const { start, end, text } = textImprovementTarget;
    if (sceneDraft.text.slice(start, end) !== text) {
      window.alert("Der Szenentext hat sich seit der Verbesserung geaendert. Bitte markiere den Text erneut.");
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

  if (!window.confirm("Aktuellen Szenentext durch diese verbesserte Fassung ersetzen?")) {
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

function openTopbarSection(section: WorkspaceSection) {
  workspace.setActiveSection(section);
}

function openTopbarWriting() {
  workspace.setActiveSection("writing");
  if (workspace.sidebarTab === "ai") {
    workspace.setSidebarTab("context");
  }
}

function openTopbarBible() {
  workspace.setActiveSection("bible");
}

function openTopbarAi() {
  workspace.setActiveSection("ai");
}

function searchResultKindLabel(kind: string, targetType?: ProjectBibleCardType | null) {
  if (kind === "scene") {
    return "Szene";
  }

  return targetType ? cardTypeSingular(targetType) : "Projektbibel";
}

function selectProjectSearchResult(result: SearchProjectResult) {
  if (result.kind === "scene" && result.sceneId) {
    manuscriptStore.selectScene(result.sceneId);
    return;
  }

  if (result.kind === "bible" && result.targetType && result.targetId) {
    projectBibleStore.setActiveCardType(result.targetType);
    projectBibleStore.selectCard(result.targetId);
    workspace.setActiveSection("bible");
  }
}

function updateSearchQuery(event: Event) {
  searchStore.setQuery((event.target as HTMLInputElement).value);
}

function normalizeUiError(error: unknown, fallback: string) {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }

  if (typeof error === "string") {
    return error;
  }

  return fallback;
}

function defaultExportFileName(format: ExportFormat) {
  const extension = format === "markdown" ? "md" : "docx";
  const projectName = projectStore.activeProject?.name || "Manuskript";
  const safeName = projectName
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "-")
    .replace(/\s+/g, " ");

  return `${safeName || "Manuskript"}.${extension}`;
}

function safeProjectFileBaseName() {
  const projectName = projectStore.activeProject?.name || "Projekt";
  return (
    projectName
      .trim()
      .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "-")
      .replace(/\s+/g, " ") || "Projekt"
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

  return `${safeProjectFileBaseName()} - Zwischenstand ${date} ${time}.autorenprojekt`;
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
    exportStore.setError(normalizeUiError(error, "Der Speichern-Dialog konnte nicht geoeffnet werden."));
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
    projectStore.setCopyError(normalizeUiError(error, "Der Speichern-Dialog konnte nicht geoeffnet werden."));
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
  const sectionMarker = /(?:^|\n)\s*(?:Erweiterte Szene|Ueberarbeitete Szene|Überarbeitete Szene|Neue Fassung|Szenentext|Text)\s*:\s*/gi;
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
      /^(Aktuelle Szene|Titel|Status|Kurzbeschreibung|Szenentext-Auszug|Projektbibel-Kontext|KI|Antwort)\s*:/i.test(
        lines[0].trim(),
      ) ||
      /^Hier ist\b.*:\s*$/i.test(lines[0].trim()))
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

  if (!window.confirm("Aktuellen Szenentext durch diese KI-Antwort ersetzen?")) {
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

  window.clearTimeout(cardSaveTimer);
  await flushCardDraftSave();
  if (bibleAiMode.value === "generate") {
    await aiStore.generateBibleCard(cardDraft.targetType, bibleAiKeywords.value);
  } else {
    await aiStore.completeBibleCard(cardDraft.targetType, cardDraft.id, bibleAiKeywords.value);
  }

  hydrateBibleAiDraft();
}

async function acceptBibleAiSuggestion() {
  const result = await aiStore.acceptActiveBibleSuggestion({ ...bibleAiDraft });
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

onMounted(async () => {
  loadStoredLayout();
  await Promise.all([projectStore.loadRecentProjects(), aiStore.loadSettings()]);

  try {
    appInfo.value = await getAppInfo();
  } catch {
    appInfoFailed.value = true;
  }
});

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
    window.clearTimeout(saveTimer);
    window.clearTimeout(cardSaveTimer);
    window.clearTimeout(searchTimer);
    clearAllSceneTextHistories();
    loadStoredNavigatorCollapse(path);
    searchStore.reset();

    if (path) {
      await Promise.all([manuscriptStore.loadTree(), projectBibleStore.load()]);
      pruneNavigatorCollapse();
      await aiStore.loadSceneSuggestions(manuscriptStore.activeSceneId);
      return;
    }

    manuscriptStore.reset();
    projectBibleStore.reset();
    aiStore.reset();
  },
);

watch(
  () => projectStore.activeProject,
  () => hydrateProjectSettings(),
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
      await aiStore.loadBibleSuggestions(projectBibleStore.activeCardType);
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
      await persistCardDraftNow();
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
    hydrateCardDraft(projectBibleStore.selectedCard);
    if (workspace.activeSection === "bible" && workspace.sidebarTab === "ai") {
      await aiStore.loadBibleSuggestions(projectBibleStore.activeCardType);
      hydrateBibleAiDraft();
    }
  },
  { immediate: true },
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
</script>

<template>
  <ProjectStartView v-if="!projectStore.activeProject" />

  <main v-else class="app-shell">
    <header class="topbar">
      <div>
        <p class="eyebrow">Autorenprogramm</p>
        <h1>{{ projectStore.activeProject?.name }}</h1>
      </div>

      <div class="topbar-actions" aria-label="Globale Aktionen">
        <button type="button" :disabled="!canUndoSceneText" @click="undoSceneText">
          Rueckgaengig
        </button>
        <button type="button" :disabled="!canRedoSceneText" @click="redoSceneText">
          Wiederholen
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'writing' && workspace.sidebarTab !== 'ai' }"
          @click="openTopbarWriting"
        >
          Schreiben
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'bible' }"
          @click="openTopbarBible"
        >
          Projektbibel
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'search' }"
          @click="openTopbarSection('search')"
        >
          Suche
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'export' }"
          @click="openTopbarSection('export')"
        >
          Export
        </button>
        <button type="button" :class="{ active: workspace.activeSection === 'ai' }" @click="openTopbarAi">
          KI
        </button>
        <button
          type="button"
          :class="{ active: workspace.activeSection === 'settings' }"
          @click="openTopbarSection('settings')"
        >
          Setup
        </button>
        <button type="button" :disabled="projectStore.isSavingCopy" @click="saveProjectCopyAs">
          Speichern unter
        </button>
        <span class="save-state" :class="{ problem: manuscriptStore.error }">{{ saveState }}</span>
        <span
          v-if="projectCopyState"
          class="save-state"
          :class="{ problem: projectStore.copyError }"
        >
          {{ projectCopyState }}
        </span>
      </div>
    </header>

    <section class="workspace" :style="workspaceStyle" aria-label="Schreibarbeitsplatz">
      <aside v-if="workspace.activeSection !== 'bible'" class="navigator" aria-label="Kapitel und Szenen">
        <div class="panel-heading">
          <h2>Navigator</h2>
          <span>{{ manuscriptStore.areas.length }} Bereiche</span>
        </div>

        <p v-if="manuscriptStore.error" class="error-text">{{ manuscriptStore.error }}</p>
        <p v-else-if="manuscriptStore.isLoading" class="muted-text">Laedt Manuskript</p>

        <ol v-else class="area-list">
          <li
            v-for="(area, areaIndex) in manuscriptStore.areas"
            :key="area.id"
            class="area-node"
            :class="{ 'drop-target': pointerDropKey === areaDropKey(area.id) }"
            data-drop-kind="area"
            :data-area-id="area.id"
            :data-area-index="areaIndex"
          >
            <div class="area-row">
              <button
                type="button"
                class="drag-handle"
                aria-label="Bereich ziehen"
                title="Bereich ziehen"
                @pointerdown.stop="startAreaPointerDrag(area, $event)"
              >
                ::
              </button>
              <button
                type="button"
                class="collapse-toggle"
                :aria-expanded="!isAreaCollapsed(area.id)"
                :aria-label="isAreaCollapsed(area.id) ? 'Bereich ausklappen' : 'Bereich einklappen'"
                :title="isAreaCollapsed(area.id) ? 'Bereich ausklappen' : 'Bereich einklappen'"
                @click.stop="toggleAreaCollapsed(area.id)"
              >
                {{ isAreaCollapsed(area.id) ? ">" : "v" }}
              </button>
              <input
                class="area-title navigator-title-input"
                :class="{ 'navigator-focused': focusedNavigatorKey === areaNavigatorKey(area.id) }"
                :value="area.title"
                :data-navigator-key="areaNavigatorKey(area.id)"
                aria-label="Bereichsname"
                @focus="
                  selectArea(area);
                  focusNavigatorItem(areaNavigatorKey(area.id));
                "
                @blur="blurNavigatorItem(areaNavigatorKey(area.id))"
                @change="updateAreaTitle(area, $event)"
                @keydown.enter.prevent="commitChapterTitle"
                @keydown="handleAreaKeydown(area, $event)"
              />
              <button
                type="button"
                class="icon-action"
                aria-label="Bereich archivieren"
                title="Bereich archivieren"
                @click="deleteArea(area)"
              >
                x
              </button>
            </div>

            <ol
              v-if="!isAreaCollapsed(area.id)"
              class="chapter-list"
              data-drop-kind="area-end"
              :data-area-id="area.id"
            >
              <li
                v-for="(chapter, chapterIndex) in area.chapters"
                :key="chapter.id"
                class="chapter-node"
                :class="{ 'drop-target': pointerDropKey === chapterDropKey(chapter.id) }"
                data-drop-kind="chapter"
                :data-area-id="area.id"
                :data-chapter-id="chapter.id"
                :data-chapter-index="chapterIndex"
              >
                <div class="chapter-row">
                  <button
                    type="button"
                    class="drag-handle"
                    aria-label="Kapitel ziehen"
                    title="Kapitel ziehen"
                    @pointerdown.stop="startChapterPointerDrag(chapter, $event)"
                  >
                    ::
                  </button>
                  <button
                    type="button"
                    class="collapse-toggle"
                    :aria-expanded="!isChapterCollapsed(chapter.id)"
                    :aria-label="
                      isChapterCollapsed(chapter.id) ? 'Kapitel ausklappen' : 'Kapitel einklappen'
                    "
                    :title="
                      isChapterCollapsed(chapter.id) ? 'Kapitel ausklappen' : 'Kapitel einklappen'
                    "
                    @click.stop="toggleChapterCollapsed(chapter.id)"
                  >
                    {{ isChapterCollapsed(chapter.id) ? ">" : "v" }}
                  </button>
                  <input
                    class="chapter-title navigator-title-input"
                    :class="{ 'navigator-focused': focusedNavigatorKey === chapterNavigatorKey(chapter.id) }"
                    :value="chapter.title"
                    :data-navigator-key="chapterNavigatorKey(chapter.id)"
                    aria-label="Kapitelname"
                    @focus="
                      selectChapter(chapter);
                      focusNavigatorItem(chapterNavigatorKey(chapter.id));
                    "
                    @blur="blurNavigatorItem(chapterNavigatorKey(chapter.id))"
                    @change="updateChapterTitle(chapter, $event)"
                    @keydown.enter.prevent="commitChapterTitle"
                    @keydown="handleChapterKeydownInArea(area, chapter, $event)"
                  />
                  <button
                    type="button"
                    class="icon-action"
                    aria-label="Kapitel archivieren"
                    title="Kapitel archivieren"
                    @click="deleteChapter(chapter)"
                  >
                    x
                  </button>
                </div>

                <ol
                  v-if="!isChapterCollapsed(chapter.id)"
                  class="scene-list"
                  data-drop-kind="chapter-end"
                  :data-chapter-id="chapter.id"
                >
                  <li
                    v-for="(scene, sceneIndex) in chapter.scenes"
                    :key="scene.id"
                    class="scene-row"
                    :class="{
                      selected: manuscriptStore.activeSceneId === scene.id,
                      'navigator-focused': focusedNavigatorKey === sceneNavigatorKey(scene.id),
                      'drop-target': pointerDropKey === sceneDropKey(chapter.id, sceneIndex),
                    }"
                    data-drop-kind="scene"
                    :data-chapter-id="chapter.id"
                    :data-scene-index="sceneIndex"
                  >
                    <button
                      type="button"
                      class="drag-handle"
                      aria-label="Szene ziehen"
                      title="Szene ziehen"
                      @pointerdown.stop="startScenePointerDrag(chapter, scene, $event)"
                    >
                      ::
                    </button>
                    <button
                      type="button"
                      class="scene-title"
                      :data-navigator-key="sceneNavigatorKey(scene.id)"
                      @focus="
                        manuscriptStore.selectScene(scene.id);
                        focusNavigatorItem(sceneNavigatorKey(scene.id));
                      "
                      @blur="blurNavigatorItem(sceneNavigatorKey(scene.id))"
                      @click="manuscriptStore.selectScene(scene.id)"
                      @keydown="handleSceneKeydown(chapter, scene, $event)"
                    >
                      {{ scene.title }}
                    </button>
                    <button
                      type="button"
                      class="icon-action"
                      aria-label="Szene archivieren"
                      title="Szene archivieren"
                      @click="deleteScene(scene)"
                    >
                      x
                    </button>
                  </li>
                  <li
                    class="scene-drop-target"
                    :class="{ 'drop-target': pointerDropKey === chapterEndDropKey(chapter.id) }"
                    data-drop-kind="chapter-end"
                    :data-chapter-id="chapter.id"
                  >
                    <button type="button" @click="createSceneFromNavigator(chapter.id)">Szene anlegen</button>
                  </li>
                </ol>
              </li>
              <li
                class="chapter-drop-target"
                :class="{ 'drop-target': pointerDropKey === areaEndDropKey(area.id) }"
                data-drop-kind="area-end"
                :data-area-id="area.id"
              >
                <button type="button" @click="createChapterFromNavigator(area.id)">Kapitel anlegen</button>
              </li>
            </ol>
          </li>
        </ol>

        <div class="navigator-actions">
          <button type="button" @click="createAreaFromNavigator">Bereich</button>
          <button type="button" @click="createChapterFromNavigator()">Kapitel</button>
          <button type="button" @click="createSceneForActiveChapter">Szene</button>
        </div>
      </aside>

      <aside v-else class="navigator bible-navigator" aria-label="Projektbibel-Karten">
        <div class="panel-heading">
          <h2>Projektbibel</h2>
          <span>{{ projectBibleStore.cards.length }} Karten</span>
        </div>

        <p v-if="projectBibleStore.error" class="error-text">{{ projectBibleStore.error }}</p>
        <p v-else-if="projectBibleStore.isLoading" class="muted-text">Laedt Projektbibel</p>

        <div class="bible-type-list" aria-label="Kartentypen">
          <button
            v-for="cardType in projectBibleCardTypes"
            :key="cardType.id"
            type="button"
            :class="{ active: projectBibleStore.activeCardType === cardType.id }"
            @click="projectBibleStore.setActiveCardType(cardType.id)"
          >
            {{ cardType.label }}
          </button>
        </div>

        <label class="navigator-search">
          <span>Suche</span>
          <input v-model="projectBibleStore.searchQuery" placeholder="Karte suchen" />
        </label>

        <ol class="bible-card-list">
          <li
            v-for="card in projectBibleStore.filteredCards"
            :key="card.id"
            class="bible-card-row"
            :class="{
              selected: selectedCard?.id === card.id,
              'navigator-focused': focusedNavigatorKey === bibleCardNavigatorKey(card.id),
            }"
          >
            <button
              type="button"
              :data-navigator-key="bibleCardNavigatorKey(card.id)"
              @focus="selectBibleCard(card)"
              @blur="blurNavigatorItem(bibleCardNavigatorKey(card.id))"
              @click="selectBibleCard(card)"
            >
              <strong>{{ card.title }}</strong>
              <span>{{ card.subtitle || cardTypeSingular(card.targetType) }}</span>
            </button>
          </li>
        </ol>

        <section v-if="!projectBibleStore.filteredCards.length" class="empty-panel compact-empty">
          <p>Keine Karten gefunden.</p>
        </section>

        <div class="navigator-actions bible-actions">
          <button type="button" @click="createBibleCard">Neue Karte</button>
        </div>
      </aside>

      <button
        type="button"
        class="resize-handle"
        aria-label="Navigator verbreitern oder verkleinern"
        @pointerdown="startPanelResize('navigator', $event)"
      />

      <section
        v-if="workspace.activeSection === 'settings'"
        class="editor settings-editor"
        aria-label="Projekteinstellungen"
      >
        <form class="settings-form" @submit.prevent="saveProjectSettings">
          <div class="settings-heading">
            <div>
              <p class="eyebrow">Setup</p>
              <h2>Projekteinstellungen</h2>
            </div>
            <button type="submit" class="primary-action" :disabled="!canSaveProjectSettings">
              Projekteinstellungen speichern
            </button>
          </div>

          <label>
            <span>Projektname</span>
            <input v-model="projectSettingsDraft.name" autocomplete="off" />
          </label>

          <label>
            <span>Beschreibung</span>
            <textarea v-model="projectSettingsDraft.description" />
          </label>

          <label>
            <span>Standardsprache</span>
            <select v-model="projectSettingsDraft.defaultLanguage">
              <option value="de">Deutsch</option>
              <option value="en">Englisch</option>
            </select>
          </label>

          <section class="settings-subsection" aria-label="Editor">
            <h3>Editor</h3>
            <label>
              <span>Schriftart</span>
              <select v-model="projectSettingsDraft.editorFontFamily">
                <option v-for="font in editorFontOptions" :key="font.id" :value="font.id">
                  {{ font.label }}
                </option>
              </select>
            </label>

            <label>
              <span>Schriftgroesse</span>
              <select v-model.number="projectSettingsDraft.editorFontSize">
                <option v-for="fontSize in editorFontSizeOptions" :key="fontSize" :value="fontSize">
                  {{ fontSize }} px
                </option>
              </select>
            </label>
          </section>

          <p v-if="projectStore.error" class="error-text">{{ projectStore.error }}</p>
        </form>

        <section class="settings-info ai-settings-panel" aria-label="Lokale KI">
          <h3>Lokale KI</h3>
          <div class="ai-provider-status">
            <label v-if="aiStore.hasModels" class="ai-model-picker">
              <span>Modell</span>
              <select
                :value="aiStore.settings?.model"
                :disabled="aiStore.isSaving"
                @change="selectAiModel"
              >
                <option value="" disabled>Modell auswaehlen</option>
                <option v-for="model in aiStore.models" :key="model.id" :value="model.name">
                  {{ modelOptionLabel(model) }}
                </option>
              </select>
            </label>
            <div v-else class="ai-readonly-model">
              <span>Modell</span>
              <strong>{{ aiConfiguredModelLabel }}</strong>
            </div>
            <small>http://localhost:11434/v1</small>
            <strong class="ai-provider-state" :class="aiStore.providerStatus?.state || 'unknown'">
              {{ aiProviderStateLabel }}
            </strong>
            <p v-if="aiStore.isLoadingModels" class="muted-text">Modelle werden geladen.</p>
            <p v-else-if="aiModelListEmpty" class="muted-text">
              Keine Ollama-Modelle installiert. Installiere oder importiere Modelle ausserhalb
              des Autorenprogramms.
            </p>
            <p v-else-if="aiModelMissing" class="error-text">
              Das konfigurierte Modell ist nicht installiert. Bitte ein vorhandenes Modell
              auswaehlen.
            </p>
            <button
              v-if="aiStore.providerStatus?.canStart"
              type="button"
              class="primary-action"
              :disabled="aiStore.isStartingProvider"
              @click="startLocalAiProvider"
            >
              {{ aiStore.isStartingProvider ? "Startet Ollama" : "Ollama starten" }}
            </button>
          </div>
          <p v-if="aiStore.error" class="error-text">{{ aiStore.error }}</p>
        </section>

        <section class="settings-info" aria-label="Projektinformationen">
          <h3>Projektdatei</h3>
          <dl>
            <div>
              <dt>Speicherort</dt>
              <dd>{{ projectStore.activeProject?.path }}</dd>
            </div>
            <div>
              <dt>Speicherart</dt>
              <dd>{{ projectStore.activeProject?.storage_kind || "file" }}</dd>
            </div>
            <div>
              <dt>Erstellt</dt>
              <dd>{{ formatProjectTimestamp(projectStore.activeProject?.created_at) }}</dd>
            </div>
            <div>
              <dt>Aktualisiert</dt>
              <dd>{{ formatProjectTimestamp(projectStore.activeProject?.updated_at) }}</dd>
            </div>
          </dl>
        </section>
      </section>

      <section
        v-else-if="workspace.activeSection === 'search'"
        class="editor utility-editor"
        aria-label="Suche"
      >
        <div class="utility-panel">
          <p class="eyebrow">Suche</p>
          <h2>Projektweite Suche</h2>
          <label class="search-field">
            <span>Suchbegriff</span>
            <input
              :value="searchStore.query"
              placeholder="Szenen und Projektbibel durchsuchen"
              autofocus
              @input="updateSearchQuery"
            />
          </label>
          <div class="search-scope-tabs" aria-label="Suchbereich">
            <button
              type="button"
              :class="{ active: searchStore.scope === 'all' }"
              @click="searchStore.setScope('all')"
            >
              Alle
            </button>
            <button
              type="button"
              :class="{ active: searchStore.scope === 'scenes' }"
              @click="searchStore.setScope('scenes')"
            >
              Szenen
            </button>
            <button
              type="button"
              :class="{ active: searchStore.scope === 'bible' }"
              @click="searchStore.setScope('bible')"
            >
              Projektbibel
            </button>
          </div>

          <p v-if="searchStore.error" class="error-text">{{ searchStore.error }}</p>
          <p v-else-if="searchStore.isLoading" class="muted-text">Suche laeuft...</p>
          <p v-else-if="!searchStore.query.trim()" class="muted-text">
            Gib einen Begriff ein, um Szenen und Projektbibel-Karten zu finden.
          </p>
          <p v-else-if="!searchStore.results.length" class="muted-text">
            Keine Treffer gefunden.
          </p>

          <section v-else class="search-results" aria-label="Suchergebnisse">
            <article
              v-for="result in searchStore.results"
              :key="result.id"
              class="search-result"
              :class="{ active: result.kind === 'scene' && result.sceneId === manuscriptStore.activeSceneId }"
            >
              <button type="button" @click="selectProjectSearchResult(result)">
                <span class="search-result-kind">
                  {{ searchResultKindLabel(result.kind, result.targetType) }}
                </span>
                <strong>{{ result.title }}</strong>
                <span>{{ result.subtitle }}</span>
              </button>
              <div class="search-result-fields">
                <span v-for="field in result.matchedFields" :key="field">{{ field }}</span>
              </div>
              <p v-for="snippet in result.snippets" :key="snippet">{{ snippet }}</p>
            </article>
          </section>
        </div>
      </section>

      <section
        v-else-if="workspace.activeSection === 'export'"
        class="editor utility-editor"
        aria-label="Export"
      >
        <div class="utility-panel">
          <p class="eyebrow">Export</p>
          <h2>Manuskript exportieren</h2>
          <div class="utility-option-grid">
            <button
              type="button"
              :disabled="exportStore.isExporting"
              @click="exportManuscript('markdown')"
            >
              Markdown exportieren
            </button>
            <button
              type="button"
              :disabled="exportStore.isExporting"
              @click="exportManuscript('docx')"
            >
              DOCX exportieren
            </button>
          </div>
          <p v-if="exportStore.error" class="error-text">{{ exportStore.error }}</p>
          <p v-else-if="exportStore.isExporting" class="muted-text">Export wird geschrieben...</p>
          <p v-else-if="exportStore.lastExportPath" class="muted-text">
            Exportiert: {{ exportStore.lastExportPath }}
          </p>
          <p class="muted-text">
            Exportiert werden aktive Bereiche und Kapitel als Ueberschriften sowie der Szenentext
            in Manuskriptreihenfolge. Szenentitel, Kurzbeschreibungen, Projektbibel und
            KI-Vorschlaege bleiben intern.
          </p>
        </div>
      </section>

      <section v-else-if="workspace.activeSection === 'ai'" class="editor ai-chat-editor" aria-label="KI-Chat">
        <div class="ai-chat-heading">
          <div>
            <p class="eyebrow">KI</p>
            <h2>Chat</h2>
          </div>
          <button type="button" @click="aiStore.clearChat">Neue Unterhaltung</button>
        </div>

        <div class="ai-chat-mode-switch" aria-label="KI-Modus">
          <button
            type="button"
            :class="{ active: aiChatMode === 'ideas' }"
            @click="aiChatMode = 'ideas'"
          >
            Ideenberater
          </button>
          <button
            type="button"
            :class="{ active: aiChatMode === 'writing' }"
            @click="aiChatMode = 'writing'"
          >
            Schreibassistent
          </button>
        </div>

        <div class="ai-chat-messages" aria-live="polite">
          <p v-if="!aiStore.chatMessages.length" class="muted-text">
            Stelle eine Frage zum Projekt, zur aktuellen Szene oder zu einer Schreibentscheidung.
          </p>
          <article
            v-for="(message, index) in aiStore.chatMessages"
            :key="`${message.role}:${index}`"
            class="ai-chat-message"
            :class="message.role"
          >
            <h3>{{ message.role === "user" ? "Du" : "KI" }}</h3>
            <p>{{ message.content }}</p>
            <div v-if="message.role === 'assistant'" class="ai-chat-message-actions">
              <button
                type="button"
                class="primary-action"
                :disabled="!canAcceptAiChatText(message.content)"
                @click="acceptAiChatMessage(message.content)"
              >
                Uebernehmen
              </button>
            </div>
          </article>
          <p v-if="aiStore.isRunningChat" class="muted-text">KI schreibt Antwort...</p>
        </div>

        <form class="ai-chat-composer" @submit.prevent="sendAiChat">
          <label class="checkbox-row">
            <input
              v-model="aiChatIncludeSceneContext"
              type="checkbox"
              :disabled="!manuscriptStore.activeSceneId"
            />
            <span>Aktuelle Szene als Kontext mitschicken</span>
          </label>
          <textarea
            v-model="aiChatDraft"
            placeholder="Frage an die KI"
            :disabled="aiStore.isRunningChat"
            @keydown="handleAiChatKeydown"
          />
          <button
            type="submit"
            class="primary-action"
            :disabled="!aiChatDraft.trim() || aiStore.isRunningChat || !aiStore.isProviderReachable"
          >
            Senden
          </button>
          <p v-if="aiStore.error" class="error-text">{{ aiStore.error }}</p>
        </form>
      </section>

      <section v-else-if="workspace.activeSection !== 'bible'" class="editor" aria-label="Aktuelle Szene">
        <template v-if="sceneDraft.id">
          <div class="scene-meta">
            <label>
              <span>Titel</span>
              <input v-model="sceneDraft.title" />
            </label>

            <label>
              <span>Status</span>
              <select v-model="sceneDraft.status">
                <option v-for="status in sceneStatuses" :key="status">{{ status }}</option>
              </select>
            </label>

            <div class="scene-info-chips" aria-label="Szeneninformationen">
              <span>{{ manuscriptStore.wordCount }} Woerter</span>
              <span>{{ manuscriptStore.activeLocation }}</span>
              <span>{{ writingBibleSceneLinks.length }} Karten</span>
            </div>
          </div>

          <textarea
            ref="sceneTextAreaRef"
            v-model="sceneDraft.text"
            :style="sceneEditorStyle"
            spellcheck="true"
            aria-label="Markdown-Editor"
            @select="updateSceneTextSelection"
            @keydown="handleSceneTextKeydown"
            @keyup="updateSceneTextSelection"
            @mouseup="updateSceneTextSelection"
          />
        </template>

        <section v-else class="empty-editor">
          <h2>Keine Szene ausgewaehlt</h2>
          <p>Lege eine Szene an, um mit dem Manuskript zu beginnen.</p>
          <button type="button" class="primary-action" @click="createSceneForActiveChapter">
            Szene anlegen
          </button>
        </section>
      </section>

      <section v-else class="editor bible-editor" aria-label="Ausgewaehlte Projektbibel-Karte">
        <template v-if="selectedCard">
          <div class="card-meta">
            <div>
              <p class="eyebrow">{{ selectedCardTypeLabel }}</p>
              <h2>{{ selectedCard.title || cardTypeSingular(selectedCard.targetType) }}</h2>
            </div>
            <button type="button" class="danger-action" @click="archiveSelectedBibleCard">
              Archivieren
            </button>
          </div>

          <div v-if="cardDraft.targetType !== 'note'" class="bible-form card-editor-form">
            <label>
              <span>Name</span>
              <input v-model="cardDraft.name" />
            </label>

            <label v-if="cardDraft.targetType === 'character'">
              <span>Rolle</span>
              <input v-model="cardDraft.role" />
            </label>

            <label>
              <span>Beschreibung</span>
              <textarea v-model="cardDraft.description" />
            </label>

            <label>
              <span>Notizen</span>
              <textarea v-model="cardDraft.notes" />
            </label>
          </div>

          <div v-else class="bible-form card-editor-form">
            <label>
              <span>Titel</span>
              <input v-model="cardDraft.title" />
            </label>

            <label>
              <span>Notiztyp</span>
              <select v-model="cardDraft.noteType">
                <option v-for="noteType in noteTypes" :key="noteType.id" :value="noteType.id">
                  {{ noteType.label }}
                </option>
              </select>
            </label>

            <label>
              <span>Inhalt</span>
              <textarea v-model="cardDraft.body" />
            </label>
          </div>
        </template>

        <section v-else class="empty-editor">
          <h2>Keine Karte ausgewaehlt</h2>
          <p>Lege eine Karte an, um Projektwissen zu erfassen.</p>
          <button type="button" class="primary-action" @click="createBibleCard">
            Karte anlegen
          </button>
        </section>
      </section>

      <button
        type="button"
        class="resize-handle"
        aria-label="Seitenleiste verbreitern oder verkleinern"
        @pointerdown="startPanelResize('sidebar', $event)"
      />

      <aside class="sidebar" aria-label="Rechte Seitenleiste">
        <div class="tabs" role="tablist" aria-label="Seitenleistenmodus">
          <button
            v-for="tab in sidebarTabs"
            :key="tab.id"
            type="button"
            :class="{ active: workspace.sidebarTab === tab.id }"
            @click="workspace.setSidebarTab(tab.id)"
          >
            {{ tab.label }}
          </button>
        </div>

        <div class="panel-heading">
          <h2>{{ sidebarTitle }}</h2>
          <span
            v-if="workspace.sidebarTab === 'ai'"
            class="panel-heading-status ai-provider-state"
            :class="aiStore.providerStatus?.state || 'unknown'"
          >
            {{ aiProviderStateLabel }}
          </span>
          <span v-else>{{ workspace.sidebarTab }}</span>
        </div>

        <template v-if="workspace.activeSection === 'bible'">
          <section v-if="workspace.sidebarTab === 'context'" class="scene-assignment-list">
            <p v-if="!selectedCard" class="muted-text">Keine Karte ausgewaehlt</p>
            <template v-else>
              <label
                v-for="scene in manuscriptStore.scenes"
                :key="scene.id"
                class="scene-assignment-row"
              >
                <input
                  type="checkbox"
                  :checked="isSceneLinkedToSelectedCard(scene.id)"
                  @change="toggleSelectedCardScene(scene.id, $event)"
                />
                <span>
                  <strong>{{ scene.title }}</strong>
                  <small>{{ scene.summary || "Keine Kurzbeschreibung" }}</small>
                </span>
              </label>
              <p v-if="!manuscriptStore.scenes.length" class="muted-text">
                Noch keine Szenen vorhanden.
              </p>
            </template>
          </section>

          <section v-else-if="workspace.sidebarTab === 'bible'" class="context-list">
            <template v-if="selectedCard">
              <h3>Typ</h3>
              <p>{{ cardTypeSingular(selectedCard.targetType) }}</p>
              <h3>Titel</h3>
              <p>{{ selectedCard.title }}</p>
              <h3>Zusammenfassung</h3>
              <p>{{ selectedCard.subtitle || "Noch keine Zusammenfassung" }}</p>
              <h3>Szenen</h3>
              <p>{{ projectBibleStore.selectedCardLinks.length }} verknuepft</p>
            </template>
            <p v-else class="muted-text">Keine Karte ausgewaehlt</p>
          </section>

          <section v-else class="ai-panel">
            <label>
              <span>Aktion</span>
              <select v-model="bibleAiMode">
                <option value="generate">Neue Karte aus Stichworten</option>
                <option value="complete">Bestehende Karte vervollstaendigen</option>
              </select>
            </label>

            <label>
              <span>Stichworte</span>
              <textarea
                v-model="bibleAiKeywords"
                placeholder="Name, Rolle, wichtige Merkmale, offene Details"
              />
            </label>

            <div class="context-list compact">
              <p>{{ selectedCard?.title || "Keine Karte" }}</p>
              <p>{{ cardTypeSingular(cardDraft.targetType) }}</p>
              <p>{{ projectBibleStore.selectedCardLinks.length }} Szenen verknuepft</p>
              <p v-if="bibleAiMode === 'complete'">
                Leere Felder: {{ bibleAiEmptyFields.length ? bibleAiEmptyFields.join(", ") : "keine" }}
              </p>
            </div>

            <button
              type="button"
              class="primary-action"
              :disabled="!canRunBibleAi"
              @click="runBibleAi"
            >
              {{ aiStore.isRunningBible ? "Erstellt Vorschlag" : "Ausfuehren" }}
            </button>

            <p v-if="aiStore.error" class="error-text">{{ aiStore.error }}</p>
            <p v-else-if="!bibleAiKeywords.trim()" class="muted-text">
              Gib Stichworte ein, bevor die Karten-KI ausgefuehrt wird.
            </p>
            <p v-else-if="!aiStore.isProviderReachable" class="muted-text">
              Die lokale KI muss erreichbar sein, bevor diese Aktion ausgefuehrt werden kann.
            </p>
            <p v-else-if="aiModelListEmpty" class="muted-text">
              Es ist noch kein lokales Modell fuer diese Aktion verfuegbar.
            </p>
            <p v-else-if="aiModelMissing" class="muted-text">
              Bitte zuerst ein installiertes Modell auswaehlen.
            </p>
            <p v-else-if="bibleAiMode === 'complete' && !selectedCard" class="muted-text">
              Bitte zuerst eine Projektbibel-Karte auswaehlen.
            </p>

            <article v-if="aiStore.activeBibleSuggestion && bibleAiHasDraft" class="ai-suggestion">
              <h3>Vorschlag</h3>
              <div v-if="cardDraft.targetType !== 'note'" class="bible-form card-editor-form">
                <label>
                  <span>Name</span>
                  <input v-model="bibleAiDraft.name" />
                </label>
                <label v-if="cardDraft.targetType === 'character'">
                  <span>Rolle</span>
                  <input v-model="bibleAiDraft.role" />
                </label>
                <label>
                  <span>Beschreibung</span>
                  <textarea v-model="bibleAiDraft.description" />
                </label>
                <label>
                  <span>Notizen</span>
                  <textarea v-model="bibleAiDraft.notes" />
                </label>
              </div>
              <div v-else class="bible-form card-editor-form">
                <label>
                  <span>Titel</span>
                  <input v-model="bibleAiDraft.title" />
                </label>
                <label>
                  <span>Notiztyp</span>
                  <select v-model="bibleAiDraft.noteType" :disabled="bibleAiMode === 'complete'">
                    <option v-for="noteType in noteTypes" :key="noteType.id" :value="noteType.id">
                      {{ noteType.label }}
                    </option>
                  </select>
                </label>
                <label>
                  <span>Inhalt</span>
                  <textarea v-model="bibleAiDraft.body" />
                </label>
              </div>
              <p v-if="bibleAiMode === 'complete'" class="muted-text">
                Beim Uebernehmen werden nur leere Felder gefuellt.
              </p>
              <p v-else-if="bibleAiDuplicateName" class="muted-text">
                Es gibt bereits eine Karte mit diesem Namen.
              </p>
              <small>{{ aiStore.activeBibleSuggestion.status }}</small>
              <div class="ai-suggestion-actions">
                <button
                  type="button"
                  class="primary-action"
                  :disabled="aiStore.activeBibleSuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="acceptBibleAiSuggestion"
                >
                  Uebernehmen
                </button>
                <button
                  type="button"
                  class="danger-action"
                  :disabled="aiStore.activeBibleSuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="rejectBibleAiSuggestion"
                >
                  Verwerfen
                </button>
              </div>
            </article>
          </section>
        </template>

        <template v-else>
          <section v-if="workspace.sidebarTab === 'context'" class="context-list">
            <h3>Aktuelle Szene</h3>
            <p>{{ sceneDraft.title || "Keine Szene" }}</p>
            <h3>Status</h3>
            <p>{{ sceneDraft.status }}</p>
            <h3>Kurzbeschreibung</h3>
            <p>{{ sceneDraft.summary || "Noch keine Kurzbeschreibung" }}</p>
          </section>

          <section v-else-if="workspace.sidebarTab === 'bible'" class="writing-bible-panel">
            <p v-if="!manuscriptStore.activeSceneId" class="muted-text">Keine Szene ausgewaehlt</p>

            <template v-else>
              <section class="writing-bible-add-panel">
                <div class="writing-bible-add-row">
                  <select v-model="writingBibleAddType" aria-label="Kartentyp">
                    <option
                      v-for="cardType in projectBibleCardTypes"
                      :key="cardType.id"
                      :value="cardType.id"
                    >
                      {{ cardType.label }}
                    </option>
                  </select>
                  <button
                    type="button"
                    class="primary-action"
                    :disabled="projectBibleStore.isSaving"
                    @click="createWritingBibleCard"
                  >
                    Neue Karte
                  </button>
                </div>

                <input
                  v-model="writingBibleSearchQuery"
                  class="writing-bible-search"
                  placeholder="Karte suchen"
                  aria-label="Karte suchen"
                />

                <div v-if="writingBibleAvailableCards.length" class="writing-bible-add-list">
                  <button
                    v-for="card in writingBibleAvailableCards"
                    :key="projectBibleCardKey(card)"
                    type="button"
                    class="writing-bible-add-card"
                    @click="addWritingBibleCard(card)"
                  >
                    <span>
                      <strong>{{ card.title || cardTypeSingular(card.targetType) }}</strong>
                      <small>{{ card.subtitle || cardTypeSingular(card.targetType) }}</small>
                    </span>
                    <small>Hinzufuegen</small>
                  </button>
                </div>
                <p v-else class="muted-text">Keine weiteren Karten</p>
              </section>

              <p v-if="!writingBibleGroups.length" class="muted-text">Keine Karten zu dieser Szene</p>

              <section
                v-for="group in writingBibleGroups"
                v-else
                :key="group.type"
                class="writing-bible-group"
              >
                <h3>{{ group.label }}</h3>
                <article
                  v-for="card in group.cards"
                  :key="projectBibleCardKey(card)"
                  class="writing-bible-card"
                  :class="{ open: isWritingBibleCardOpen(card) }"
                >
                  <button type="button" @click="toggleWritingBibleCard(card)">
                    <span>{{ card.title || cardTypeSingular(card.targetType) }}</span>
                    <small>{{ card.subtitle || cardTypeSingular(card.targetType) }}</small>
                  </button>

                  <div v-if="isWritingBibleCardOpen(card)" class="writing-bible-card-detail">
                    <div
                      v-if="cardDraft.id === card.id && cardDraft.targetType === card.targetType"
                      class="bible-form writing-bible-card-form"
                    >
                      <template v-if="cardDraft.targetType !== 'note'">
                        <label>
                          <span>Name</span>
                          <input v-model="cardDraft.name" />
                        </label>
                        <label v-if="cardDraft.targetType === 'character'">
                          <span>Rolle</span>
                          <input v-model="cardDraft.role" />
                        </label>
                        <label>
                          <span>Beschreibung</span>
                          <textarea v-model="cardDraft.description" />
                        </label>
                        <label>
                          <span>Notizen</span>
                          <textarea v-model="cardDraft.notes" />
                        </label>
                      </template>

                      <template v-else>
                        <label>
                          <span>Titel</span>
                          <input v-model="cardDraft.title" />
                        </label>
                        <label>
                          <span>Notiztyp</span>
                          <select v-model="cardDraft.noteType">
                            <option
                              v-for="noteType in noteTypes"
                              :key="noteType.id"
                              :value="noteType.id"
                            >
                              {{ noteType.label }}
                            </option>
                          </select>
                        </label>
                        <label>
                          <span>Inhalt</span>
                          <textarea v-model="cardDraft.body" />
                        </label>
                      </template>
                    </div>
                  </div>
                </article>
              </section>
            </template>
          </section>

          <section v-else class="ai-panel">
            <label>
              <span>Aktion</span>
              <select v-model="sidebarAiAction">
                <option value="summary">Szene zusammenfassen</option>
                <option value="contradictions">Widersprueche pruefen</option>
                <option value="alternatives">Synonyme / Umformulierungen</option>
                <option value="improve">Text verbessern</option>
              </select>
            </label>

            <div class="context-list compact">
              <p v-for="line in aiContextPreview" :key="line">{{ line }}</p>
            </div>

            <template v-if="sidebarAiAction === 'summary'">
              <button
                type="button"
                class="primary-action"
                :disabled="!canRunSceneSummary"
                @click="runSceneSummary"
              >
                {{ aiStore.isRunning ? "Fasst zusammen" : "Ausfuehren" }}
              </button>
            </template>

            <template v-else-if="sidebarAiAction === 'contradictions'">
              <button
                type="button"
                class="primary-action"
                :disabled="!canRunSceneContradictions"
                @click="runSceneContradictions"
              >
                {{ aiStore.isRunning ? "Prueft Widersprueche" : "Widersprueche pruefen" }}
              </button>
            </template>

            <template v-else-if="sidebarAiAction === 'alternatives'">
              <div class="context-list compact">
                <p v-if="selectedSceneText">Markierung: {{ selectedSceneTextLabel }}</p>
                <p v-else>Markiere ein Wort oder einen Satz im Szenentext.</p>
              </div>

              <button
                type="button"
                class="primary-action"
                :disabled="!canRunTextAlternatives"
                @click="runTextAlternatives"
              >
                {{ aiStore.isRunningAlternatives ? "Sucht Alternativen" : "Alternativen erzeugen" }}
              </button>
            </template>

            <template v-else>
              <div class="context-list compact">
                <p v-if="selectedSceneText">Markierung: {{ selectedSceneTextLabel }}</p>
                <p v-else>Keine Markierung: Ganze Szene wird verbessert.</p>
              </div>

              <button
                type="button"
                class="primary-action"
                :disabled="!canRunTextImprovements"
                @click="runTextImprovements"
              >
                {{ aiStore.isRunningImprovements ? "Verbessert Text" : "Text verbessern" }}
              </button>
            </template>

            <p v-if="aiStore.error" class="error-text">{{ aiStore.error }}</p>
            <p v-else-if="sidebarAiAction === 'summary' && !sceneDraft.text.trim()" class="muted-text">
              Die Szene braucht Text, bevor sie zusammengefasst werden kann.
            </p>
            <p v-else-if="sidebarAiAction === 'contradictions' && !sceneDraft.text.trim()" class="muted-text">
              Die Szene braucht Text, bevor sie geprueft werden kann.
            </p>
            <p
              v-else-if="sidebarAiAction === 'contradictions' && !writingBibleSceneLinks.length"
              class="muted-text"
            >
              Verknuepfe zuerst mindestens eine Projektbibel-Karte mit dieser Szene.
            </p>
            <p v-else-if="sidebarAiAction === 'alternatives' && !selectedSceneText" class="muted-text">
              Markiere zuerst Text im Szeneneditor.
            </p>
            <p v-else-if="sidebarAiAction === 'improve' && !sceneDraft.text.trim()" class="muted-text">
              Die Szene braucht Text, bevor sie verbessert werden kann.
            </p>
            <p v-else-if="!aiStore.isProviderReachable" class="muted-text">
              Die lokale KI muss erreichbar sein, bevor diese Aktion ausgefuehrt werden kann.
            </p>
            <p v-else-if="aiModelListEmpty" class="muted-text">
              Es ist noch kein lokales Modell fuer diese Aktion verfuegbar.
            </p>
            <p v-else-if="aiModelMissing" class="muted-text">
              Bitte zuerst ein installiertes Modell auswaehlen.
            </p>

            <div
              v-if="sidebarAiAction === 'alternatives' && aiStore.textAlternatives.length"
              class="ai-alternatives"
            >
              <article
                v-for="alternative in aiStore.textAlternatives"
                :key="alternative"
                class="ai-suggestion"
              >
                <p>{{ alternative }}</p>
                <div class="ai-suggestion-actions">
                  <button
                    type="button"
                    class="primary-action"
                    @click="applyTextAlternative(alternative)"
                  >
                    Einsetzen
                  </button>
                </div>
              </article>
            </div>

            <div
              v-if="sidebarAiAction === 'improve' && aiStore.textImprovements.length"
              class="ai-alternatives"
            >
              <article
                v-for="improvement in aiStore.textImprovements"
                :key="improvement"
                class="ai-suggestion"
              >
                <p>{{ improvement }}</p>
                <div class="ai-suggestion-actions">
                  <button
                    type="button"
                    class="primary-action"
                    @click="applyTextImprovement(improvement)"
                  >
                    Einsetzen
                  </button>
                </div>
              </article>
            </div>

            <article
              v-if="sidebarAiAction === 'summary' && activeSceneSummarySuggestion"
              class="ai-suggestion"
            >
              <h3>Vorschlag</h3>
              <p>{{ activeSceneSummarySuggestion.result }}</p>
              <small>{{ activeSceneSummarySuggestion.status }}</small>
              <div class="ai-suggestion-actions">
                <button
                  type="button"
                  class="primary-action"
                  :disabled="activeSceneSummarySuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="acceptAiSummary"
                >
                  Uebernehmen
                </button>
                <button
                  type="button"
                  class="danger-action"
                  :disabled="activeSceneSummarySuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="rejectSceneAiSuggestion(activeSceneSummarySuggestion.id)"
                >
                  Verwerfen
                </button>
              </div>
            </article>

            <article
              v-if="sidebarAiAction === 'contradictions' && activeSceneContradictionSuggestion"
              class="ai-suggestion"
            >
              <h3>Pruefergebnis</h3>
              <p>{{ activeSceneContradictionSuggestion.result }}</p>
              <small>{{ activeSceneContradictionSuggestion.status }}</small>
              <div class="ai-suggestion-actions single">
                <button
                  type="button"
                  class="danger-action"
                  :disabled="activeSceneContradictionSuggestion.status !== 'draft' || aiStore.isSaving"
                  @click="rejectSceneAiSuggestion(activeSceneContradictionSuggestion.id)"
                >
                  Verwerfen
                </button>
              </div>
            </article>
          </section>
        </template>
      </aside>
    </section>

    <footer class="statusbar">
      <template v-if="workspace.activeSection === 'bible'">
        <span>Karten {{ bibleSaveState.toLowerCase() }}</span>
        <span>{{ projectBibleStore.cards.length }} {{ selectedCardTypeLabel.toLowerCase() }}</span>
        <span>{{ projectBibleStore.selectedCardLinks.length }} Szenen verknuepft</span>
      </template>
      <template v-else-if="workspace.activeSection === 'settings'">
        <span>Projekteinstellungen</span>
        <span>{{ projectSettingsDraft.defaultLanguage }}</span>
        <span>{{ projectStore.activeProject?.path }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'search'">
        <span>{{ searchStore.query.trim() ? `${searchStore.results.length} Treffer` : "Suche" }}</span>
        <span>{{ manuscriptStore.scenes.length }} Szenen</span>
        <span>{{ searchStore.isLoading ? "laeuft" : "bereit" }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'export'">
        <span>{{ exportStore.isExporting ? "Export laeuft" : "Export bereit" }}</span>
        <span>{{ manuscriptStore.areas.length }} Bereiche</span>
        <span>{{ exportStore.lastExportPath || "Markdown / DOCX" }}</span>
      </template>
      <template v-else-if="workspace.activeSection === 'ai'">
        <span>KI-Chat</span>
        <span>{{ aiStore.chatMessages.length }} Nachrichten</span>
        <span>{{ aiStore.providerStatus?.state || "unbekannt" }}</span>
      </template>
      <template v-else>
        <span>Strg+S {{ saveState.toLowerCase() }}</span>
        <span>{{ manuscriptStore.wordCount }} Woerter</span>
        <span>{{ manuscriptStore.activeLocation }}</span>
      </template>
      <span v-if="appInfo">{{ appInfo.name }} {{ appInfo.version }} / {{ appInfo.environment }}</span>
      <span v-else-if="appInfoFailed">Backend nicht verbunden</span>
      <button type="button" @click="workspace.resetLayout()">Layout zuruecksetzen</button>
    </footer>
  </main>
</template>

<style>
:root {
  color: #1d2322;
  background: #f2f0eb;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 16px;
  font-synthesis: none;
  line-height: 1.5;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-width: 1180px;
  min-height: 720px;
}

button,
input,
select,
textarea {
  font: inherit;
}

button {
  border: 1px solid transparent;
  background: transparent;
  color: inherit;
  cursor: pointer;
}

input,
select,
textarea {
  width: 100%;
  border: 1px solid #c8c5bd;
  border-radius: 6px;
  background: #fbfaf7;
  color: #1d2322;
}

input,
select {
  min-height: 36px;
  padding: 6px 10px;
}

textarea {
  min-height: 120px;
  padding: 12px;
  resize: none;
}

.app-shell {
  display: grid;
  grid-template-rows: 72px minmax(0, 1fr) 34px;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 12px 20px;
  border-bottom: 1px solid #d7d2c7;
  background: #fbfaf7;
}

.topbar > div:first-child {
  min-width: 0;
}

.topbar h1,
.panel-heading h2 {
  margin: 0;
  letter-spacing: 0;
}

.topbar h1 {
  font-size: 22px;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.eyebrow {
  margin: 0 0 2px;
  color: #6c746e;
  font-size: 12px;
  font-weight: 650;
  text-transform: uppercase;
}

.topbar-actions {
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  min-width: 0;
  justify-content: flex-end;
}

.topbar-actions button,
.navigator-actions button,
.tabs button,
.primary-action,
.danger-action,
.chapter-drop-target button,
.scene-drop-target button {
  min-height: 34px;
  border-color: #c9c4b8;
  border-radius: 6px;
  background: #ffffff;
  padding: 6px 12px;
}

.topbar-actions button {
  white-space: nowrap;
}

.topbar-actions button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.topbar-actions button.active {
  border-color: #7fa28d;
  background: #dce8df;
  color: #193d31;
  font-weight: 700;
}

.save-state {
  color: #2d6a54;
  font-size: 13px;
  font-weight: 650;
}

.save-state.problem,
.error-text {
  color: #9b2d25;
}

.workspace {
  display: grid;
  min-height: 0;
}

.resize-handle {
  width: 6px;
  min-width: 6px;
  height: 100%;
  border: 0;
  border-radius: 0;
  background: #d7d2c7;
  padding: 0;
  cursor: col-resize;
}

.resize-handle:hover,
.resize-handle:focus-visible {
  background: #7fa28d;
  outline: none;
}

.navigator,
.sidebar {
  min-width: 0;
  border-right: 1px solid #d7d2c7;
  background: #ede9df;
}

.tabs button.active,
.scene-row.selected {
  background: #dce8df;
  color: #193d31;
  font-weight: 650;
}

.navigator,
.sidebar {
  padding: 16px;
  overflow: auto;
}

.sidebar {
  border-right: 0;
  border-left: 1px solid #d7d2c7;
}

.panel-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
}

.panel-heading h2 {
  font-size: 16px;
  font-weight: 700;
}

.panel-heading span,
.statusbar,
.context-list h3 {
  color: #6c746e;
  font-size: 12px;
  font-weight: 650;
  text-transform: uppercase;
}

.panel-heading .panel-heading-status {
  min-width: 0;
  overflow-wrap: anywhere;
  text-align: right;
  text-transform: none;
}

.muted-text,
.error-text {
  margin: 8px 0;
  font-size: 13px;
  font-weight: 650;
}

.area-list,
.area-list ol {
  list-style: none;
  margin: 0;
  padding: 0;
}

.area-node,
.chapter-node {
  margin-bottom: 8px;
}

.area-node {
  --navigator-indent: 0px;
}

.chapter-node {
  --navigator-indent: 22px;
}

.scene-row,
.scene-drop-target {
  --navigator-indent: 44px;
}

.chapter-drop-target {
  --navigator-indent: 22px;
}

.area-row,
.chapter-row,
.scene-row {
  display: grid;
  grid-template-columns: 24px 22px minmax(0, 1fr) 24px;
  align-items: center;
  gap: 4px;
  border-radius: 6px;
  margin-left: var(--navigator-indent);
  width: calc(100% - var(--navigator-indent));
}

.scene-row {
  grid-template-columns: 24px minmax(0, 1fr) 24px;
}

.area-row {
  margin-top: 12px;
  margin-bottom: 7px;
}

.chapter-list {
  margin-left: 0;
}

.chapter-row {
  margin-top: 8px;
  margin-bottom: 5px;
}

.scene-list {
  margin-left: 0;
}

.area-title,
.chapter-title,
.scene-title {
  min-width: 0;
  min-height: 30px;
  border-radius: 6px;
  overflow: hidden;
  padding: 5px 8px;
  text-align: left;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.area-title {
  color: #31483d;
  font-size: 17px;
  font-weight: 750;
}

.chapter-title {
  color: #4c554f;
  font-size: 15px;
  font-weight: 650;
}

.navigator-title-input {
  height: 30px;
  border-color: transparent;
  background: transparent;
}

.navigator-title-input:hover,
.navigator-title-input:focus,
.navigator-title-input.navigator-focused {
  border-color: #c8c5bd;
  background: #fbfaf7;
  outline: none;
}

.scene-title {
  padding-left: 8px;
  font-size: 14px;
  font-weight: 450;
}

.scene-row {
  min-height: 30px;
  margin-bottom: 4px;
}

.icon-action {
  width: 24px;
  min-width: 24px;
  height: 28px;
  border-radius: 6px;
  color: #6c746e;
  padding: 0;
}

.collapse-toggle {
  width: 22px;
  min-width: 22px;
  height: 28px;
  border-color: transparent;
  border-radius: 6px;
  background: transparent;
  color: #4f665a;
  font-size: 12px;
  font-weight: 750;
  padding: 0;
}

.drag-handle {
  width: 24px;
  min-width: 24px;
  height: 28px;
  border-radius: 6px;
  color: #6c746e;
  font-size: 12px;
  font-weight: 700;
  padding: 0;
  cursor: grab;
  touch-action: none;
  user-select: none;
}

.drag-handle:active {
  cursor: grabbing;
}

.icon-action:hover,
.icon-action:focus-visible,
.drag-handle:hover,
.drag-handle:focus-visible {
  background: #f8d7d2;
  color: #7b2018;
  outline: none;
}

.collapse-toggle:hover,
.collapse-toggle:focus-visible {
  border-color: #b9c8be;
  background: #dce8df;
  color: #193d31;
  outline: none;
}

.icon-action:disabled {
  cursor: not-allowed;
  opacity: 0.3;
}

.area-node.drop-target > .area-row,
.chapter-node.drop-target > .chapter-row,
.scene-row.drop-target,
.chapter-drop-target.drop-target,
.scene-drop-target.drop-target,
.scene-row.navigator-focused {
  outline: 2px solid #5d8c72;
  outline-offset: 2px;
  background: #dce8df;
}

.chapter-drop-target,
.scene-drop-target {
  margin: 6px 0 0 var(--navigator-indent);
  width: calc(100% - var(--navigator-indent));
}

.chapter-drop-target button,
.scene-drop-target button {
  width: 100%;
  min-height: 30px;
  color: #4f665a;
  font-size: 13px;
  padding: 4px 8px;
  text-align: left;
}

.navigator-actions {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-top: 18px;
}

.editor {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-width: 0;
  padding: 18px 22px;
  background: #f8f6f0;
}

.scene-meta {
  display: grid;
  grid-template-columns: minmax(220px, 1fr) minmax(130px, 160px) minmax(220px, auto);
  align-items: end;
  gap: 12px;
  padding-bottom: 12px;
}

.scene-info-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  min-width: 0;
}

.scene-info-chips span {
  min-height: 28px;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  color: #59625c;
  padding: 5px 8px;
  font-size: 12px;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

label span {
  display: block;
  margin-bottom: 5px;
  color: #59625c;
  font-size: 12px;
  font-weight: 650;
}

.editor > textarea {
  min-height: 0;
  border-color: #d4cfc4;
  background: #fffdfa;
  line-height: 1.75;
}

.settings-editor {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 18px;
  overflow: auto;
}

.settings-form,
.settings-info {
  display: grid;
  gap: 14px;
  width: 100%;
  max-width: 760px;
}

.settings-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.settings-heading h2 {
  margin: 0;
  font-size: 24px;
  letter-spacing: 0;
}

.settings-subsection {
  display: grid;
  gap: 14px;
  border-top: 1px solid #d7d2c7;
  padding-top: 16px;
}

.settings-subsection h3 {
  margin: 0;
  color: #31483d;
  font-size: 14px;
  font-weight: 750;
  text-transform: uppercase;
}

.settings-form textarea {
  height: 120px;
}

.settings-info {
  border-top: 1px solid #d7d2c7;
  padding-top: 16px;
}

.settings-info h3 {
  margin: 0;
  color: #31483d;
  font-size: 14px;
  font-weight: 750;
  text-transform: uppercase;
}

.settings-info dl {
  display: grid;
  gap: 8px;
  margin: 0;
}

.settings-info dl div {
  display: grid;
  grid-template-columns: 120px minmax(0, 1fr);
  gap: 12px;
}

.settings-info dt {
  color: #6c746e;
  font-size: 13px;
  font-weight: 700;
}

.settings-info dd {
  min-width: 0;
  margin: 0;
  overflow-wrap: anywhere;
}

.utility-editor,
.ai-chat-editor {
  gap: 16px;
  overflow: auto;
}

.utility-panel {
  display: grid;
  align-self: start;
  gap: 14px;
  max-width: 760px;
}

.utility-panel h2,
.ai-chat-heading h2 {
  margin: 0;
  color: #1d2322;
  font-size: 24px;
  letter-spacing: 0;
}

.utility-option-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 160px));
  gap: 10px;
}

.utility-option-grid button {
  min-height: 42px;
  border: 1px solid #1f5f49;
  border-radius: 6px;
  background: #28664f;
  color: #ffffff;
  font-weight: 700;
}

.utility-option-grid button:disabled {
  cursor: wait;
  opacity: 0.65;
}

.search-scope-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.search-scope-tabs button {
  border: 1px solid #c9c4b8;
  border-radius: 999px;
  background: #ffffff;
  color: #4f665a;
  padding: 7px 12px;
  font-size: 13px;
  font-weight: 750;
}

.search-scope-tabs button.active {
  border-color: #28664f;
  background: #dfece4;
  color: #123b2c;
}

.search-results {
  display: grid;
  gap: 10px;
}

.search-result {
  display: grid;
  gap: 8px;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fffdfa;
  padding: 12px;
}

.search-result.active {
  border-color: #7fa28d;
  background: #edf5ef;
}

.search-result button {
  display: grid;
  gap: 2px;
  width: 100%;
  padding: 0;
  text-align: left;
}

.search-result button strong {
  color: #1d2322;
  font-size: 18px;
}

.search-result button span,
.search-result p {
  margin: 0;
  color: #59625c;
}

.search-result button .search-result-kind {
  justify-self: start;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  color: #4f665a;
  padding: 2px 7px;
  font-size: 12px;
  font-weight: 750;
}

.search-result-fields {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.search-result-fields span {
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  color: #4f665a;
  padding: 3px 7px;
  font-size: 12px;
  font-weight: 700;
}

.ai-chat-editor {
  grid-template-rows: auto auto minmax(0, 1fr) auto;
}

.ai-chat-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.ai-chat-heading p {
  margin: 0;
  color: #6c746e;
  font-size: 13px;
  font-weight: 650;
}

.ai-chat-heading button {
  min-height: 34px;
  border: 1px solid #c9c4b8;
  border-radius: 6px;
  background: #ffffff;
  padding: 6px 12px;
}

.ai-chat-mode-switch {
  display: inline-grid;
  grid-template-columns: repeat(2, minmax(0, 180px));
  width: fit-content;
  overflow: hidden;
  border: 1px solid #c9c4b8;
  border-radius: 6px;
  background: #ffffff;
}

.ai-chat-mode-switch button {
  min-height: 36px;
  border-radius: 0;
  padding: 6px 12px;
  color: #4f5954;
  font-weight: 700;
}

.ai-chat-mode-switch button + button {
  border-left: 1px solid #c9c4b8;
}

.ai-chat-mode-switch button.active {
  background: #dce8df;
  color: #193d31;
}

.ai-chat-messages {
  display: grid;
  align-content: start;
  gap: 10px;
  min-height: 0;
  overflow: auto;
  border: 1px solid #d4cfc4;
  border-radius: 6px;
  background: #fffdfa;
  padding: 12px;
}

.ai-chat-message {
  display: grid;
  gap: 4px;
  max-width: 82%;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  padding: 9px 10px;
}

.ai-chat-message.user {
  justify-self: end;
  background: #dce8df;
}

.ai-chat-message.assistant {
  justify-self: start;
}

.ai-chat-message h3 {
  margin: 0;
  color: #59625c;
  font-size: 12px;
  font-weight: 750;
  text-transform: uppercase;
}

.ai-chat-message p {
  margin: 0;
  white-space: pre-wrap;
}

.ai-chat-message-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 4px;
}

.ai-chat-message-actions .primary-action {
  min-height: 30px;
  padding: 4px 10px;
  font-size: 13px;
}

.ai-alternatives {
  display: grid;
  gap: 8px;
}

.ai-chat-composer {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
}

.ai-chat-composer textarea {
  grid-column: 1 / -1;
  min-height: 92px;
  resize: vertical;
}

.ai-chat-composer .error-text {
  grid-column: 1 / -1;
}

.checkbox-row {
  display: flex;
  grid-column: 1 / -1;
  align-items: center;
  gap: 8px;
}

.checkbox-row input {
  width: auto;
}

.checkbox-row span {
  margin: 0;
}

.empty-editor {
  align-self: center;
  justify-self: center;
  max-width: 360px;
  text-align: center;
}

.empty-editor h2 {
  margin: 0 0 8px;
  font-size: 20px;
}

.empty-editor p {
  margin: 0 0 16px;
  color: #59625c;
}

.tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
  margin-bottom: 16px;
}

.tabs button {
  padding-inline: 6px;
}

.context-list p {
  min-height: 32px;
  margin: 0 0 7px;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  padding: 6px 9px;
}

.context-list h3 {
  margin: 16px 0 6px;
}

.compact {
  margin: 14px 0;
}

.bible-form,
.ai-panel {
  display: grid;
  gap: 12px;
}

.ai-provider-status {
  display: grid;
  gap: 6px;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  padding: 8px 10px;
}

.ai-provider-status span {
  font-size: 13px;
  font-weight: 700;
}

.ai-provider-status > strong,
.ai-readonly-model strong {
  min-width: 0;
  overflow-wrap: anywhere;
}

.ai-readonly-model {
  display: grid;
  gap: 5px;
}

.ai-model-picker {
  display: grid;
  gap: 5px;
}

.ai-provider-status small,
.ai-suggestion small {
  color: #6c746e;
  font-size: 12px;
}

.ai-provider-state {
  color: #59625c;
  font-size: 12px;
}

.ai-provider-state.reachable {
  color: #2d6a54;
}

.ai-provider-state.unreachable,
.ai-provider-state.unsupported,
.ai-provider-state.error {
  color: #9b2d25;
}

.ai-provider-state.starting {
  color: #7b5a1e;
}

.ai-suggestion {
  display: grid;
  gap: 8px;
  border: 1px solid #c9c4b8;
  border-radius: 6px;
  background: #fffdfa;
  padding: 10px;
}

.ai-suggestion h3 {
  margin: 0;
  color: #31483d;
  font-size: 13px;
  font-weight: 750;
  text-transform: uppercase;
}

.ai-suggestion p {
  margin: 0;
  white-space: pre-wrap;
}

.ai-suggestion-actions {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.ai-suggestion-actions.single {
  grid-template-columns: minmax(0, 1fr);
}

.bible-form textarea {
  height: 150px;
}

.writing-bible-panel {
  display: grid;
  gap: 14px;
}

.writing-bible-add-panel {
  display: grid;
  gap: 8px;
  border-bottom: 1px solid #d7d2c7;
  padding-bottom: 12px;
}

.writing-bible-add-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
}

.writing-bible-add-row select,
.writing-bible-search {
  min-width: 0;
  min-height: 34px;
  border: 1px solid #c9c4b8;
  border-radius: 6px;
  background: #ffffff;
  padding: 6px 8px;
}

.writing-bible-add-list {
  display: grid;
  gap: 6px;
  max-height: 180px;
  overflow: auto;
}

.writing-bible-add-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
  min-height: 40px;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  padding: 7px 9px;
  text-align: left;
}

.writing-bible-add-card span,
.writing-bible-card button span {
  min-width: 0;
}

.writing-bible-add-card strong,
.writing-bible-card button span {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.writing-bible-add-card small,
.writing-bible-card button small {
  display: block;
  overflow: hidden;
  color: #6c746e;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.writing-bible-add-card:hover,
.writing-bible-add-card:focus-visible {
  border-color: #7fa28d;
  background: #dce8df;
  outline: none;
}

.writing-bible-group h3,
.writing-bible-card-detail h4 {
  margin: 0;
  color: #6c746e;
  font-size: 12px;
  font-weight: 750;
  text-transform: uppercase;
}

.writing-bible-group {
  display: grid;
  gap: 6px;
}

.writing-bible-card {
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  overflow: hidden;
}

.writing-bible-card button {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 2px;
  width: 100%;
  min-height: 38px;
  padding: 8px 10px;
  text-align: left;
  font-weight: 700;
}

.writing-bible-card.open button {
  background: #dce8df;
  color: #193d31;
}

.writing-bible-card-detail {
  display: grid;
  gap: 6px;
  padding: 0 10px 10px;
}

.writing-bible-card-form textarea {
  height: 96px;
}

.writing-bible-card-detail p {
  margin: 0 0 6px;
  color: #1d2322;
  white-space: pre-wrap;
}

.primary-action {
  background: #265d49;
  color: #ffffff;
  font-weight: 700;
}

.primary-action:disabled {
  background: #7d8b84;
  cursor: not-allowed;
}

.danger-action {
  background: #fff7f4;
  color: #8a2b22;
  font-weight: 700;
}

.bible-type-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
  margin-bottom: 14px;
}

.bible-type-list button {
  min-height: 34px;
  border: 1px solid #c9c4b8;
  border-radius: 6px;
  background: #ffffff;
  padding: 6px 8px;
  text-align: left;
}

.bible-type-list button.active,
.bible-card-row.selected button {
  background: #dce8df;
  color: #193d31;
  font-weight: 650;
}

.navigator-search {
  display: block;
  margin-bottom: 12px;
}

.bible-card-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.bible-card-row {
  margin-bottom: 6px;
}

.bible-card-row button {
  display: grid;
  width: 100%;
  min-height: 48px;
  border-radius: 6px;
  padding: 7px 9px;
  text-align: left;
}

.bible-card-row strong,
.scene-assignment-row strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.bible-card-row span,
.scene-assignment-row small {
  overflow: hidden;
  color: #6c746e;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.bible-card-row.navigator-focused button {
  outline: 2px solid #5d8c72;
  outline-offset: 2px;
}

.compact-empty {
  color: #6c746e;
  font-size: 13px;
}

.bible-actions {
  grid-template-columns: 1fr;
}

.bible-editor {
  grid-template-rows: auto minmax(0, 1fr);
}

.card-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding-bottom: 16px;
}

.card-meta h2 {
  margin: 0;
  font-size: 22px;
}

.card-editor-form {
  align-content: start;
  overflow: auto;
}

.card-editor-form textarea {
  min-height: 170px;
}

.scene-assignment-list {
  display: grid;
  gap: 8px;
}

.scene-assignment-row {
  display: grid;
  grid-template-columns: 20px minmax(0, 1fr);
  align-items: start;
  gap: 8px;
  border: 1px solid #d6d1c7;
  border-radius: 6px;
  background: #fbfaf7;
  padding: 8px;
}

.scene-assignment-row input {
  width: 16px;
  margin-top: 2px;
}

.scene-assignment-row span {
  display: grid;
  min-width: 0;
}

.statusbar {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 0 16px;
  border-top: 1px solid #d7d2c7;
  background: #fbfaf7;
}

.statusbar button {
  margin-left: auto;
  border: 0;
  color: #4f665a;
  font-size: 12px;
  font-weight: 650;
  padding: 0;
}
</style>
