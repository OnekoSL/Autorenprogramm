# Testprotokoll 0.2.0 – Deutsch

Stand: 24. Juli 2026

## Umgebung

- Betriebssystem: Windows 10 Pro 22H2, Build 19045.7548, x64
- Anwendung: Autorenprogramm 0.2.0, lokale Tauri-Release-Binary
- KI-Dienst: Ollama Desktop 0.32.0
- Modell: `autoren-qwen3-6-12b-iq:latest`
- Testprojekt: `C:\Users\nukun\AppData\Local\Temp\Autorenprogramm-0.2.0-test-20260722\projects\release-test-de.autorenprojekt`
- Projektkopie: `C:\Users\nukun\AppData\Local\Temp\Autorenprogramm-0.2.0-test-20260722\projects\release-test-de-copy.autorenprojekt`

## Automatische Prüfungen

| Prüfung | Ergebnis |
| --- | --- |
| `npm.cmd ci` | Bestanden |
| `npm.cmd run check:version` | Bestanden |
| `npm.cmd run check:version -- v0.2.0` | Bestanden |
| `npm.cmd run check:i18n` | Bestanden: 294 Katalogschlüssel, 276 UI-Texte, 67 Fehlercodes |
| `npm.cmd run test:coverage` | Bestanden: 25 Tests |
| `npm.cmd run build` | Bestanden |
| `cargo fmt --check` | Bestanden |
| `cargo clippy --locked --all-targets -- -D warnings` | Bestanden |
| `cargo test --locked` | Bestanden: 119 Tests |
| `npx.cmd tauri build --bundles nsis` | Bestanden |

## Manueller deutscher Ablauf

| Bereich | Ergebnis | Beobachtung |
| --- | --- | --- |
| Projekt erstellen, schließen und wieder öffnen | Bestanden | Projektsprache Deutsch und neutrale Nutzerdaten blieben erhalten. |
| Projektkopie | Bestanden | Standardname `Release Test DE - Zwischenstand …`. |
| Manuskript | Bestanden | Bereiche, Kapitel und Szenen wurden erstellt, umbenannt und archiviert. Undo, Redo, Autosave und schneller Szenenwechsel funktionierten. Echtes Drag-and-drop ordnete Szene, Kapitel und Bereich neu. Ein exklusiver SQLite-Schreib-Lock erzeugte den lokalisierten `storage_error`, verhinderte den Szenenwechsel und erhielt den Entwurf. Nach Freigabe blieb der Navigator sichtbar, der erneute Wechsel funktionierte und speicherte den Entwurf. |
| Projektbibel | Bestanden | Figuren, Orte, Gegenstände, Notizen, Fraktionen und Ereignisse wurden angelegt, Beziehungen gespeichert und eine Figur mit der Szene verknüpft. |
| Worldbuilding | Bestanden | Weltsystem, Szenenlink und die Beziehung `verbunden mit Atlas Figure` wurden gespeichert. |
| Struktur | Bestanden | Dreiaktstruktur, Beat-Text, Status sowie Szenen- und Figurenlink wurden gespeichert. |
| Suche | Bestanden | Gesamtsuche und Bereichsfilter fanden die erwarteten neutralen Testdaten. |
| Markdown- und DOCX-Export | Bestanden | Lokalisierte Namen `Release Test DE - Manuskript.*`; Reihenfolge und Inhalt wurden geprüft. |
| Einstellungen und Sprache | Bestanden | `Deutsch` war schreibgeschützt, Version `0.2.0` sichtbar, Einzahl und Mehrzahl wurden korrekt angezeigt. |
| Offline-KI | Bestanden | `Lokale KI nicht erreichbar` wurde deutsch angezeigt; die übrigen Offline-Funktionen blieben nutzbar. |
| KI-Chat ohne und mit Szenenkontext | Bestanden nach Korrektur | Antworten waren deutsch, Rohdetails blieben verborgen und die Übernahme blieb bestätigungspflichtig. |
| Szenen-KI | Bestanden | Zusammenfassung, Widerspruchsprüfung, Textverbesserung und Alternativen erzeugten nicht automatisch übernommene Vorschläge. |
| Projektbibel-KI | Bestanden nach Korrektur | Erzeugen und Bearbeiten lieferten bestätigungspflichtige Kartenentwürfe. |

## Während des Tests behobene Fehler

1. Exportdateien enthielten noch nicht den lokalisierten Bestandteil `Manuskript`.
2. Denkmodelle konnten ihr Tokenbudget vollständig für internes Reasoning verbrauchen.
3. Eine zweite Systemnachricht verursachte mit Qwen3 bei Szenenkontext HTTP 400.
4. Ollamas loser JSON-Modus reichte für den Kartenentwurf nicht aus; kartentypspezifische JSON-Schemata stabilisieren die Ausgabe.
5. Der Projektstart verwendete noch keinen lokalisierten Bestandteil `Projekt`.
6. Im normalen Chat konnte das Modell `<think>`-Blöcke in das sichtbare Inhaltsfeld schreiben; diese werden nun entfernt.
7. Im Worldbuilding wurde eine zuvor ausgewählte Projektbibel-Karte fälschlich aus der Beziehungszielliste entfernt; außerdem blieb die Seitenleistenüberschrift im englischen Projekt deutsch. Beide Fehler wurden korrigiert und im frischen Build erneut geprüft.
8. Nach einem fehlgeschlagenen Manuskript-Flush blendete die Fehleranzeige den Navigatorbaum aus. Der Baum bleibt nun neben der Meldung sichtbar; Abbruch, Entwurfserhalt und erfolgreicher Wiederholungswechsel wurden mit einem echten SQLite-Schreib-Lock geprüft.

## Artefakt

- Installer: `Autorenprogramm_0.2.0_x64-setup.exe`
- Größe: 4.217.736 Byte
- Datei- und Produktversion: 0.2.0
- SHA-256: `0bb0a164daa4a1b4a15cb062b0d70a8ea6810bc5cac03f14af3f4e6812cbeca8`

## Freigabestatus

Die lokale 0.2.0-Testversion wurde mit allen während des Tests gefundenen Korrekturen neu gebaut und ist für den vorgesehenen lokalen Testumfang **releasebereit**. Die vollständigen deutschen und britisch-englischen Kernabläufe einschließlich der Randprüfungen wurden bestanden. Es erfolgten kein Commit, Tag, Push und keine Veröffentlichung. Die vor dem Test gesicherten persönlichen KI- und Zuletzt-geöffnet-Einstellungen wurden anschließend bytegenau wiederhergestellt; Ollama Desktop läuft wieder.
