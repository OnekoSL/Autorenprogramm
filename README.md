# Autorenprogramm

Autorenprogramm ist eine lokale Windows-Desktop-App fuer komplexe Romanprojekte. Der MVP verbindet Manuskriptstruktur, Szeneneditor, Projektbibel, Suche, Export und eine optionale lokale Schreibassistenz ueber Ollama.

> **Frueher MVP-Teststand:** Version 0.1.0 ist ein unsigniertes Pre-release fuer freiwillige Tests. Verwende Kopien oder Testprojekte und sichere wichtige Projektdateien regelmaessig.

## Enthaltene Funktionen

- Romanprojekte als lokale `.autorenprojekt`-Dateien anlegen und oeffnen
- Bereiche, Kapitel und Szenen strukturieren und bearbeiten
- Figuren, Orte, Gegenstaende und Notizen in der Projektbibel pflegen
- Projektweite Suche sowie Markdown- und DOCX-Export
- Projektkopien als Backup speichern
- Optional: lokale KI fuer Schreibchat, Textverbesserung, Szenenzusammenfassung und Widerspruchspruefung

Den fachlichen Umfang beschreibt [MVP Version 1](doc/02-mvp-v1.md).

## Systemvoraussetzungen

- Windows 10 oder Windows 11, x64
- Microsoft Edge WebView2 Runtime; Windows installiert sie normalerweise bereits
- Fuer KI-Funktionen: [Ollama](https://ollama.com/) und mindestens ein lokal installiertes Textmodell

Ollama und Modelle sind nicht Bestandteil des Installers. Alle Funktionen ausser der KI koennen ohne Ollama verwendet werden.

## Installation

1. Im neuesten [GitHub Release](https://github.com/OnekoSL/Autorenprogramm/releases/latest) `Autorenprogramm_0.1.0_x64-setup.exe` und die gleichnamige `.sha256`-Datei herunterladen.
2. Optional die SHA-256-Pruefsumme in PowerShell vergleichen:

   ```powershell
   Get-FileHash .\Autorenprogramm_0.1.0_x64-setup.exe -Algorithm SHA256
   ```

3. Installer starten. Der MVP ist noch nicht digital signiert. Windows SmartScreen kann deshalb warnen. Nur fortfahren, wenn Dateiname, Release-Quelle und Pruefsumme stimmen; dazu gegebenenfalls **Weitere Informationen** und **Trotzdem ausfuehren** waehlen.
4. Die Evaluierungslizenz lesen und die Installation fuer den aktuellen Benutzer abschliessen.

## Erster Test

1. Ein neues Projekt in einem selbst gewaehlten Ordner anlegen.
2. Kapitel, Szenen und Projektbibel-Eintraege erstellen und miteinander verknuepfen.
3. Unter Einstellungen regelmaessig **Projektkopie speichern** verwenden.
4. Manuskript testweise als Markdown und DOCX exportieren.
5. Fuer lokale KI Ollama starten, mindestens ein Modell installieren und dieses in den App-Einstellungen auswaehlen.

Die vollstaendige Testliste steht in [TESTING.md](TESTING.md). Fehler und Feedback koennen ueber die [GitHub Issues](https://github.com/OnekoSL/Autorenprogramm/issues) gemeldet werden.

## Daten und Datenschutz

Projektdateien bleiben am vom Nutzer gewaehlten Speicherort. Die App hat keine Telemetrie und keinen Cloud-Dienst. KI-Anfragen gehen ausschliesslich an Ollama unter `http://localhost:11434`; eine Eingabe von Cloud-URLs oder API-Keys ist in V0.1.0 nicht vorgesehen. Details stehen in [PRIVACY.md](PRIVACY.md).

## Deinstallation und Updates

Die App kann ueber die Windows-Einstellungen deinstalliert werden. Selbst angelegte `.autorenprojekt`-Dateien und Exporte werden dabei nicht geloescht. V0.1.0 besitzt keinen automatischen Updater; neue Testversionen werden als separate GitHub Releases bereitgestellt.

## Entwicklung

Vorausgesetzt werden Node.js, Rust und die [Tauri-Voraussetzungen fuer Windows](https://v2.tauri.app/start/prerequisites/).

```powershell
npm.cmd ci
npm.cmd run tauri dev
```

Pruefungen und lokaler Installer-Build:

```powershell
npm.cmd run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
npx.cmd tauri build --bundles nsis
```

## Lizenz

Der Quellcode ist sichtbar, aber nicht Open Source. Er darf ausschliesslich gemaess der [Autorenprogramm-Evaluierungslizenz](LICENSE.md) genutzt werden. Copyright (c) 2026 OnekoSL.
