# 0.2.0 test report – English

Date: 24 July 2026

## Environment

- Operating system: Windows 10 Pro 22H2, build 19045.7548, x64
- Application: Autorenprogramm 0.2.0, local Tauri release binary
- AI service: Ollama Desktop 0.32.0
- Model: `autoren-qwen3-6-12b-iq:latest`
- Test project: `C:\Users\nukun\AppData\Local\Temp\Autorenprogramm-0.2.0-test-20260722\projects\release-test-en.autorenprojekt`
- Project copy: `C:\Users\nukun\AppData\Local\Temp\Autorenprogramm-0.2.0-test-20260722\projects\release-test-en-copy.autorenprojekt`

## Automated gates

| Gate | Result |
| --- | --- |
| `npm.cmd ci` | Passed |
| Both `check:version` invocations | Passed |
| `npm.cmd run check:i18n` | Passed: 294 catalogue keys, 276 UI texts, 67 error codes |
| `npm.cmd run test:coverage` | Passed: 25 tests |
| Frontend build | Passed |
| Rust formatting and Clippy | Passed |
| Rust tests | Passed: 119 tests |
| NSIS build | Passed |

## Manual British-English flow

| Area | Result | Observation |
| --- | --- | --- |
| Create, close and reopen project | Passed | `English (UK)` and neutral user content were retained. |
| Project copy | Passed | Default name used `Release Test EN - Snapshot …`. |
| Manuscript | Passed | Part, chapter and scene were renamed; status, autosave and rapid switching retained the text. Real pointer drag-and-drop succeeded for a scene, chapter and part. An exclusive SQLite write lock produced the localised `storage_error`, aborted the scene switch and retained the draft. `Scene Alpha`, `Chapter Atlas` and `Area Atlas` were archived separately and disappeared from the navigator. |
| Story bible | Passed | All six card types were created and retained. A card relationship and scene link were saved, and card AI generation and editing produced confirmation-only drafts. |
| Worldbuilding and structure | Passed after correction | A world system was created, edited and linked to a scene and character. The English `World system` heading and relationship target list were verified in a fresh build. The three-act template, beat text, status, scene link and character link were saved. |
| Search | Passed | Searching for `Atlas 42` returned `Scene Alpha` and `1 RESULT`. |
| Markdown and DOCX export | Passed | Default names used `Release Test EN - Manuscript.*`; files were created and contained the scene in manuscript order. |
| Settings and language | Passed | `English (UK)` was read-only and version 0.2.0 was visible. |
| Offline AI | Passed | `Local AI is unavailable` appeared in English while ordinary project functions remained available. |
| AI chat | Passed after fixes | Context chat and the repeated no-context chat returned British English without visible thinking text. |
| Scene AI actions | Passed | Summary and contradiction checking returned appropriate English results for a short narrative scene. Whole-scene improvement and selected-text alternatives produced confirmation-only suggestions. |
| Story-bible AI | Passed | English generation and editing of the existing test character returned confirmation-only drafts. |

## Defects fixed during the run

The run corrected localised manuscript and project filenames, Qwen3 reasoning control, scene-context message ordering, strict Ollama JSON schemas, removal of visible `<think>` blocks, a filtered Worldbuilding relationship target, the untranslated English Worldbuilding heading, and a navigator tree hidden by a failed manuscript flush. The final corrections were verified in freshly built applications; the relationship filtering is also covered by a frontend regression test.

## Artifact

- Installer: `Autorenprogramm_0.2.0_x64-setup.exe`
- Size: 4,217,736 bytes
- File and product version: 0.2.0
- SHA-256: `0bb0a164daa4a1b4a15cb062b0d70a8ea6810bc5cac03f14af3f4e6812cbeca8`

## Release status

The local 0.2.0 test build has been rebuilt with all corrections found during testing and is **release-ready** for the intended local test scope. The complete German and British-English core flows, including the remaining edge cases, passed. No commit, tag, push or release was created. The original personal AI settings and recent-project list were restored byte for byte after the test, and Ollama Desktop is running again.
