# Lokal-First-Betriebsmodell

## Produktentscheidung

Manuskript, Projektbibel, Notizen, KI-Vorschläge und Exporte gehören dem Autor und liegen standardmäßig auf dessen Rechner. Normales Schreiben, Strukturieren, Suchen, Sichern und Exportieren funktioniert ohne Internetverbindung und ohne Benutzerkonto.

## Lokale Daten

- Jedes Projekt ist eine vom Nutzer platzierte `.autorenprojekt`-Datei auf SQLite-Basis.
- Projektkopien und Exporte werden nur nach einer ausdrücklichen Dateiauswahl geschrieben.
- Eigene Projektdateien lassen sich mit üblichen Datei-Backups sichern und archivieren.
- Eine Deinstallation entfernt keine selbst angelegten Projekte oder Exporte.

## Lokale KI in 0.2.0

KI ist optional und wird nicht mit der App ausgeliefert. Version 0.2.0 verbindet sich ausschließlich mit einem separat installierten Ollama unter `http://localhost:11434`. Nutzer installieren und verwalten Modelle außerhalb der App und wählen in den Einstellungen bewusst ein erkanntes Modell aus.

Die App bietet in diesem Release weder Cloud-KI noch frei konfigurierbare Provider oder API-Schlüssel-Speicherung an. Ohne erreichbares Ollama oder ohne ausgewähltes Modell bleiben KI-Aktionen deaktiviert und die übrigen Funktionen voll nutzbar.

## Konsequenzen

- Autoren behalten die Kontrolle über Speicherort und Backups.
- Die App besitzt keine Telemetrie und keinen eigenen Onlinedienst.
- Lokale KI benötigt passende Hardware und kann je nach Modell langsam reagieren.
- Geräte-Synchronisation, Zusammenarbeit und automatische Updates sind nicht Teil von 0.2.0.
- Zukünftige Integrationen dürfen das lokale Projektformat und den Offline-Kern nicht von einem Cloud-Dienst abhängig machen.
