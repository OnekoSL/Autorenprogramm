# Technische Architektur

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

Die Vue-Oberflaeche ruft das Backend ausschliesslich ueber typisierte TypeScript-Wrapper und Tauri Commands auf. Pinia haelt den geladenen Arbeits- und UI-Zustand, ist aber nicht die dauerhafte Quelle der Wahrheit. Das Rust-Backend validiert Eingaben und fuehrt alle SQLite-Operationen aus.

Projektdateien enthalten Manuskriptstruktur, Szenentexte, Metadaten, Projektbibel-Verknuepfungen und gespeicherte KI-Vorschlaege. Zuletzt verwendete Projektpfade und das ausgewaehlte Ollama-Modell liegen in der lokalen App-Konfiguration.

## Fachbereiche

- Projektverwaltung: Projekte anlegen, oeffnen, aktualisieren und als Kopie sichern
- Manuskript: Bereiche, Kapitel, Szenen, Reihenfolge, Status und Text
- Projektbibel: Figuren, Orte, Gegenstaende, Notizen und Szenenverknuepfungen
- Suche: Manuskript, Szenendaten und Projektbibel
- Export: deterministische Ausgabe als Markdown oder DOCX
- KI: Kontextaufbau, lokale Ollama-Aufrufe und gespeicherte Vorschlaege

## Sicherheits- und Architekturregeln

- Das Frontend fuehrt keine SQL-Abfragen aus.
- Projekt-, Such-, Projektbibel-, Backup- und Exportfunktionen arbeiten ohne Netzwerk.
- V0.1.0 akzeptiert keine Cloud-URL und keinen API-Key.
- KI-Aufrufe werden im Rust-Backend ausschliesslich an den lokalen Ollama-Loopback gesendet.
- KI-Ergebnisse sind Vorschlaege und ueberschreiben Manuskript oder Projektbibel nicht automatisch.
- Die Tauri-CSP erlaubt nur gebuendelte Inhalte und die fuer Tauri erforderliche IPC-Verbindung.

## Release

Windows-x64 ist die einzige Plattform des ersten MVP-Releases. GitHub Actions prueft Frontend und Rust-Tests, baut den NSIS-Installer und veroeffentlicht ihn mit SHA-256-Pruefsumme. V0.1.0 ist nicht digital signiert und besitzt keinen automatischen Updater.

Details zum lokalen Betrieb stehen in [15 Lokal-First-Betriebsmodell](15-lokal-first-betriebsmodell.md).
