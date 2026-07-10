# Datenschutz und lokale Datenhaltung

## Kurzfassung

Autorenprogramm V0.1.0 arbeitet lokal, verwendet keine Telemetrie und besitzt keinen eigenen Cloud-Dienst.

## Projekt- und Konfigurationsdaten

- Projekte werden als `.autorenprojekt`-Dateien an dem vom Nutzer gewaehlten Ort gespeichert.
- Exporte und Projektkopien werden nur an explizit ausgewaehlte Ziele geschrieben.
- Eine lokale Konfigurationsdatei speichert die Liste zuletzt verwendeter Projektpfade und den Namen des ausgewaehlten Ollama-Modells.
- Die App speichert keine API-Keys und bietet keine frei konfigurierbaren externen KI-Endpunkte an.

## Lokale KI

KI-Funktionen senden den fuer die jeweilige Aktion beschriebenen Szenen-, Text- oder Projektbibel-Kontext ausschliesslich an das lokale Ollama unter `http://localhost:11434`. Die App uebertraegt diesen Inhalt nicht an OnekoSL oder einen Cloud-Anbieter.

Ollama und lokale Modelle werden getrennt von dieser App installiert und betrieben. Fuer deren Downloads, Konfiguration und Verhalten gelten die Hinweise der jeweiligen Anbieter.

## Fehlerberichte

GitHub Issues sind oeffentlich. Keine Manuskripte, Projektdateien, persoenlichen Daten oder vertraulichen Logausgaben an ein Issue anhaengen. Sicherheitsprobleme bitte gemaess [SECURITY.md](SECURITY.md) privat melden.
