# Datenschutz und lokale Datenhaltung

## Kurzfassung

Autorenprogramm 0.2.0 arbeitet lokal, verwendet keine Telemetrie und besitzt keinen eigenen Cloud-Dienst.

## Projekt- und Konfigurationsdaten

- Projekte werden als `.autorenprojekt`-Dateien an dem vom Nutzer gewählten Ort gespeichert.
- Exporte und Projektkopien werden nur an ausdrücklich ausgewählte Ziele geschrieben.
- Lokale Konfigurationsdateien speichern zuletzt verwendete Projektpfade, Projektvorgaben, Layoutdaten und den Namen des ausgewählten Ollama-Modells.
- Die App speichert keine API-Schlüssel und bietet keine frei konfigurierbaren externen KI-Endpunkte an.

## Lokale KI

KI-Funktionen senden den für die jeweilige Aktion beschriebenen Szenen-, Text- oder Projektbibel-Kontext ausschließlich an das lokale Ollama unter `http://localhost:11434`. Die App überträgt diesen Inhalt nicht an OnekoSL oder einen Cloud-Anbieter.

Ollama und lokale Modelle werden getrennt von dieser App installiert und betrieben. Für deren Downloads, Konfiguration und Verhalten gelten die Hinweise der jeweiligen Anbieter.

## Fehlerberichte

GitHub Issues sind öffentlich. Keine Manuskripte, Projektdateien, persönlichen Daten oder vertraulichen Logausgaben an ein Issue anhängen. Sicherheitsprobleme bitte gemäß [SECURITY.md](SECURITY.md) privat melden.
