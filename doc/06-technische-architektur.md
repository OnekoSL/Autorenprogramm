# Technische Architektur

Die Internationalisierung verwendet `vue-i18n`, sprachneutrale Backend-IDs und projektgebundene Ressourcen für `de-DE` und `en-GB`. Fehler- und Providerstatus werden über stabile Codes übersetzt; technische Meldungen bleiben Fallback beziehungsweise Diagnoseinformation. Das persistente Schema 16 ist in [21 Mehrsprachigkeit](21-mehrsprachigkeit.md) beschrieben.

## MVP-Stack

Autorenprogramm ist eine lokale Desktop-App mit Tauri 2. Das Frontend verwendet Vue 3, TypeScript, Vite und Pinia. Das Rust-Backend kapselt Fachlogik, SQLite-Datenzugriff, Suche, Export und KI-Kontextaufbau.

- Desktop-Shell: Tauri 2
- Backend: Rust
- Frontend: Vue 3 und TypeScript
- Frontend-State: Pinia
- Persistenz: SQLite in einer lokalen `.autorenprojekt`-Datei
- Export: Markdown und DOCX im Rust-Backend
- Lokale KI: Ollama auf `http://localhost:11434/v1`

## Schichten und Datenfluss

Die Vue-Oberfläche ruft das Backend ausschließlich über typisierte TypeScript-Wrapper und Tauri-Befehle auf. Pinia hält den geladenen Arbeits- und UI-Zustand, ist aber nicht die dauerhafte Quelle der Wahrheit. Das Rust-Backend validiert Eingaben und führt alle SQLite-Operationen aus.

`App.vue` enthält nur den Anwendungsstart und die Umschaltung zwischen Startansicht und `WorkspaceShell`. Die Shell verantwortet globale Navigation und das gemeinsame Drei-Spalten-Layout. Schreiben, Projektbibel, Struktur, Worldbuilding, Suche, Export, KI und Einstellungen sind eigenständige Views. Temporärer Fachzustand und Auswahlwechsel werden über Composables und Pinia-Stores verwaltet; ein zentraler Navigationsvertrag führt vor einem Bereichswechsel den registrierten Autosave-Flush aus.

Das Rust-Modul `project` ist eine stabile interne Fassade für die in `lib.rs` registrierten Tauri-Befehle. Dahinter liegen getrennte Module für Projektverwaltung, Schema und Migrationen, Manuskript, Suche, Export, Projektbibel, Worldbuilding sowie die KI-Bereiche Provider, Einstellungen, Szenenanalyse und Projektbibel. `story_structure` bleibt ein eigenständiges Modul. Befehlsnamen, serialisierte Typen und Projektdateiformat werden durch diese Aufteilung nicht verändert.

Projektdateien enthalten Manuskriptstruktur, Szenentexte, Metadaten, Projektbibel-Verknüpfungen und gespeicherte KI-Vorschläge. Zuletzt verwendete Projektpfade und das ausgewählte Ollama-Modell liegen in der lokalen App-Konfiguration.

## Fachbereiche

- Projektverwaltung: Projekte anlegen, öffnen, aktualisieren und als Kopie sichern
- Manuskript: Bereiche, Kapitel, Szenen, Reihenfolge, Status und Text
- Projektbibel: Figuren, Orte, Gegenstände, Notizen und Szenenverknüpfungen
- Worldbuilding: Weltsysteme, eingebaute Vorlagen sowie gemeinsame Szenen- und Kartenbeziehungen
- Suche: Manuskript, Szenendaten und Projektbibel
- Export: deterministische Ausgabe als Markdown oder DOCX
- KI: Kontextaufbau, lokale Ollama-Aufrufe und gespeicherte Vorschläge

## Sicherheits- und Architekturregeln

- Das Frontend führt keine SQL-Abfragen aus.
- Projekt-, Such-, Projektbibel-, Backup- und Exportfunktionen arbeiten ohne Netzwerk.
- Version 0.2.0 akzeptiert keine Cloud-URL und keinen API-Schlüssel.
- KI-Aufrufe werden im Rust-Backend ausschließlich an den lokalen Ollama-Loopback gesendet.
- KI-Ergebnisse sind Vorschläge und überschreiben Manuskript oder Projektbibel nicht automatisch.
- Die Tauri-CSP erlaubt nur gebündelte Inhalte und die für Tauri erforderliche IPC-Verbindung.

## Release

Windows-x64 ist die einzige Plattform der Testversion 0.2.0. GitHub Actions prüft Frontend und Rust-Tests, baut den NSIS-Installer und veröffentlicht ihn mit SHA-256-Prüfsumme. Version 0.2.0 ist nicht digital signiert und besitzt keinen automatischen Updater.

Details zum lokalen Betrieb stehen in [15 Lokal-First-Betriebsmodell](15-lokal-first-betriebsmodell.md).
