# Testleitfaden für Autorenprogramm 0.2.0

[Deutsch](TESTING.md) | [English](TESTING.en.md)

Teste mit neuen oder kopierten Projekten ohne wichtige Originaldaten. Notiere Windows-Version, App-Version, Projektsprache, verwendetes Ollama-Modell und die genauen Schritte bis zu einem Fehler.

## Installation und Version

- SHA-256-Prüfsumme des Installers mit der Release-Datei vergleichen.
- Dateiname, App-Anzeige und Installer auf Version 0.2.0 prüfen.
- SmartScreen-Hinweis und Evaluierungslizenz kontrollieren.
- Installation, erster App-Start und zweiter App-Start prüfen.

## Zweisprachige Testmatrix

Den gesamten Kernablauf einmal mit einem deutschen und einmal mit einem englischen Projekt durchführen. Für beide Projekte dieselben neutralen Testinhalte verwenden. Nutzertexte müssen nach Speichern, Neustart und erneutem Öffnen unverändert bleiben.

### Projekt und Manuskript

- Projekt mit der vorgesehenen Projektsprache anlegen, schließen und erneut öffnen.
- Prüfen, dass die Projektsprache später nicht geändert werden kann.
- Bereiche, Kapitel und Szenen erstellen, umbenennen, verschieben und archivieren.
- Szenentext, Status und Kurzbeschreibung bearbeiten; Undo/Redo und Autosave prüfen.
- Während einer Speicherung schnell die Szene beziehungsweise den Bereich wechseln und prüfen, dass die jüngste Änderung erhalten bleibt.
- Projektkopie anlegen und als eigenständiges Projekt öffnen.

### Projektbibel, Worldbuilding und Struktur

- Figur, Ort, Gegenstand, Notiz, Fraktion und Ereignis erstellen und bearbeiten.
- Karten untereinander in beide Anzeigerichtungen verknüpfen und Szenen zuordnen.
- Ein Weltsystem erstellen, bearbeiten sowie mit einer Szene und einer Projektbibel-Karte verbinden.
- Eine Story-Struktur anwenden, Beats bearbeiten und mit Szenen sowie Figuren verbinden.
- Prüfen, dass Struktur- und Beziehungslinks keine Nutzertexte automatisch verändern.

### Suche, Export und Einstellungen

- Szenentext, Szenenmetadaten und alle Projektbibel-Typen suchen; Bereichsfilter prüfen.
- Markdown- und DOCX-Export mit mehreren Exportprofilen erzeugen und Reihenfolge, Umlaute beziehungsweise englische Texte prüfen.
- Lokalisierte Export- und Projektkopie-Dateinamen prüfen.
- Schriftfamilie, Schriftgröße, Zahlen, Zähler, Statusmeldungen und Fehlercodes kontrollieren.
- App neu starten und Layout, Projektinhalt, Sprache und Einstellungen erneut prüfen.

## Lokale KI

1. Ollama beenden oder unerreichbar machen. Lokalisierte Fehlermeldungen prüfen und sicherstellen, dass alle Nicht-KI-Funktionen weiterarbeiten.
2. Ollama starten und in beiden Projekten dasselbe Modell `autoren-qwen3-6-12b-iq:latest` auswählen.
3. Schreibchat mit und ohne Szenenkontext verwenden.
4. Markierten Text verbessern und genau eine Variante übernehmen.
5. Eine Szene zusammenfassen und den Vorschlag annehmen beziehungsweise ablehnen.
6. Mit verknüpftem Projektbibel-Kontext eine Widerspruchsprüfung ausführen.
7. Eine Projektbibel-Karte erzeugen, ergänzen und bearbeiten; Änderungen nur feldweise übernehmen.
8. Prüfen, dass Antworten ohne anderssprachige Nutzeranweisung der Projektsprache folgen und dass kein Vorschlag automatisch Inhalte überschreibt.

## Fehlerbericht

Im passenden GitHub-Issue App-Version, Windows-Version, Projektsprache, Ollama-Modell, genaue Schritte, erwartetes und tatsächliches Ergebnis angeben. Projekt- oder Manuskriptinhalte vor dem Einfügen anonymisieren.
