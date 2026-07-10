# Lokal-First-Betriebsmodell

## Produktentscheidung

Manuskript, Projektbibel, Notizen, KI-Vorschlaege und Exporte gehoeren dem Autor und liegen standardmaessig auf dessen Rechner. Normales Schreiben, Strukturieren, Suchen, Sichern und Exportieren funktioniert ohne Internetverbindung und ohne Benutzerkonto.

## Lokale Daten

- Jedes Projekt ist eine vom Nutzer platzierte `.autorenprojekt`-Datei auf SQLite-Basis.
- Projektkopien und Exporte werden nur nach einer ausdruecklichen Dateiauswahl geschrieben.
- Eigene Projektdateien lassen sich mit ueblichen Datei-Backups sichern und archivieren.
- Eine Deinstallation entfernt keine selbst angelegten Projekte oder Exporte.

## Lokale KI im MVP

KI ist optional und wird nicht mit der App ausgeliefert. V0.1.0 verbindet sich ausschliesslich mit einem separat installierten Ollama unter `http://localhost:11434`. Nutzer installieren und verwalten Modelle ausserhalb der App und waehlen in den Einstellungen bewusst ein erkanntes Modell aus.

Die App bietet in diesem Release weder Cloud-KI noch frei konfigurierbare Provider oder API-Key-Speicherung an. Ohne erreichbares Ollama oder ohne ausgewaehltes Modell bleiben KI-Aktionen deaktiviert und die uebrigen Funktionen voll nutzbar.

## Konsequenzen

- Autoren behalten die Kontrolle ueber Speicherort und Backups.
- Die App besitzt keine Telemetrie und keinen eigenen Onlinedienst.
- Lokale KI benoetigt passende Hardware und kann je nach Modell langsam reagieren.
- Geraete-Synchronisation, Zusammenarbeit und automatische Updates sind nicht Teil von V0.1.0.
- Zukuenftige Integrationen duerfen das lokale Projektformat und den Offline-Kern nicht zur Voraussetzung eines Cloud-Dienstes machen.
