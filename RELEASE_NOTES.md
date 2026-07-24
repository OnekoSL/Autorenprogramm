# Autorenprogramm v0.2.0

## Deutsch

Version 0.2.0 ist eine zweisprachige, unsignierte Testversion für Windows 10/11 x64. Sie erweitert den ersten öffentlichen MVP um vernetzte Planung und eine projektgebundene deutsche oder britisch-englische Oberfläche.

### Neu und verbessert

- Neue Projekte werden dauerhaft auf Deutsch oder britisches Englisch festgelegt. Oberfläche, Standardnamen, Vorlagen, Dateinamen, Formate und die Standardsprache der KI folgen dieser Projektsprache; vorhandene Nutzertexte werden nicht übersetzt.
- Die Projektbibel unterstützt Figuren, Orte, Gegenstände, Notizen, Fraktionen und Ereignisse sowie gerichtete Beziehungen und Szenenverknüpfungen.
- Ein eigener Worldbuilding-Bereich verwaltet Weltsysteme mit Regeln, Möglichkeiten, Kosten, Grenzen, Auswirkungen und Verknüpfungen.
- Story-Strukturen besitzen bearbeitbare Akte und Beats mit Status sowie Szenen- und Figurenlinks.
- Autosave und Bereichswechsel wurden gegen konkurrierende Speicherungen und den Verlust der jüngsten Änderung abgesichert.
- Bekannte Fehler werden mit lokalisierten Meldungen und stabilen Fehlercodes angezeigt; technische Rohdetails bleiben verborgen.
- Frontend und Rust-Backend wurden modularisiert und durch Frontendtests, Coverage, Rustfmt und Clippy als verpflichtende Qualitätsprüfungen abgesichert.

### Vor dem Test beachten

- Der Installer ist nicht digital signiert; Windows SmartScreen kann warnen.
- Ollama und mindestens ein lokales Textmodell müssen für KI-Funktionen separat installiert werden.
- Es gibt noch keinen automatischen Updater.
- Wichtige Projekte sollten vor dem Öffnen gesichert werden. Für den ersten Test werden neue oder kopierte Projekte empfohlen.
- Die mitgelieferte SHA-256-Datei dient zur Prüfung des Installers.

Die deutsche Testliste steht in [TESTING.md](TESTING.md).

---

## English

Version 0.2.0 is a bilingual, unsigned test build for Windows 10/11 x64. It extends the first public MVP with connected planning tools and a project-specific German or British-English interface.

### New and improved

- New projects are permanently set to German or British English. The interface, default names, templates, generated filenames, formatting and the AI's default response language follow the project language; existing user-written text is never translated.
- The project bible supports characters, locations, items, notes, factions and events, including directional relationships and scene links.
- A dedicated worldbuilding workspace manages world systems with rules, capabilities, costs, limits, effects and links.
- Story structures provide editable acts and beats with statuses, scene links and character links.
- Autosave and workspace changes are protected against concurrent saves and loss of the latest edit.
- Known failures are displayed with localised messages and stable error codes while raw technical details remain hidden.
- The frontend and Rust backend have been modularised and are protected by frontend tests, coverage, Rust formatting and Clippy quality gates.

### Before testing

- The installer is not digitally signed; Windows SmartScreen may show a warning.
- Ollama and at least one local text model must be installed separately for AI features.
- There is no automatic updater yet.
- Back up important projects before opening them. New or copied projects are recommended for initial testing.
- Use the supplied SHA-256 file to verify the installer.

The English test checklist is available in [TESTING.en.md](TESTING.en.md).
