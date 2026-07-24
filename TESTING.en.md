# Test guide for Autorenprogramm 0.2.0

[Deutsch](TESTING.md) | [English](TESTING.en.md)

Use new or copied projects without important original data. Record the Windows version, application version, project language, Ollama model and the exact steps leading to a defect.

## Installation and version

- Compare the installer's SHA-256 checksum with the release file.
- Verify version 0.2.0 in the filename, application display and installer.
- Check the SmartScreen warning and evaluation licence.
- Test installation, the first application start and a second application start.

## Bilingual test matrix

Complete the entire core workflow once with a German project and once with an English project. Use the same language-neutral test content in both projects. User-written text must remain unchanged after saving, restarting and reopening.

### Project and manuscript

- Create a project in the intended project language, close it and reopen it.
- Verify that the project language cannot be changed later.
- Create, rename, move and archive parts, chapters and scenes.
- Edit scene text, status and summary; test undo/redo and autosave.
- Change scenes or workspaces quickly during a save and verify that the latest edit is retained.
- Save a project copy and open it as an independent project.

### Project bible, worldbuilding and structure

- Create and edit a character, location, item, note, faction and event.
- Link cards and inspect both display directions; assign them to scenes.
- Create and edit a world system, then link it to a scene and a project-bible card.
- Apply a story structure, edit beats and link them to scenes and characters.
- Verify that structure and relationship links never modify user-written text automatically.

### Search, export and settings

- Search scene text, scene metadata and every project-bible type; test scope filters.
- Produce Markdown and DOCX exports with multiple profiles and check ordering, German characters and English text.
- Verify localised export and project-copy filenames.
- Check font family, font size, numbers, counters, status messages and error codes.
- Restart the application and recheck the layout, project content, language and settings.

## Local AI

1. Stop Ollama or make it unavailable. Verify localised errors and confirm that every non-AI feature still works.
2. Start Ollama and select the same `autoren-qwen3-6-12b-iq:latest` model in both projects.
3. Use writing chat with and without scene context.
4. Improve selected text and apply exactly one alternative.
5. Summarise a scene and accept or reject the suggestion.
6. Run a contradiction check with linked project-bible context.
7. Generate, complete and edit a project-bible card; apply changes field by field only.
8. Verify that responses follow the project language unless the user explicitly requests another language and that suggestions never overwrite content automatically.

## Defect report

Include the application version, Windows version, project language, Ollama model, exact steps, expected result and actual result in the relevant GitHub issue. Anonymise project or manuscript content before attaching it.
