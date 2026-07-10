# MVP Version 1

## Ziel

Version 1 liefert den ersten beweisbaren Produktkern: Ein Autor kann ein komplexes Romanprojekt lokal anlegen, Kapitel und Szenen strukturieren, Szenen schreiben, Projektwissen pflegen, Szenenkontext nutzen, KI mit gezieltem Kontext anwenden und das Manuskript exportieren.

Der MVP wird zuerst an den Beduerfnissen von Autoren komplexer Romane gemessen, besonders Fantasy und Science-Fiction.

## Verbindlicher MVP-Kern

Der MVP ist ein zusammenhaengender Arbeitsfluss, keine lose Feature-Sammlung:

- Projekt anlegen und lokal speichern.
- Kapitel und Szenen im Navigator erstellen, sortieren und bearbeiten.
- Szenentext im Editor schreiben und speichern.
- Figuren, Orte, Gegenstaende und Notizen als Projektbibel-Kernobjekte pflegen.
- Projektbibel-Eintraege mit Szenen verknuepfen.
- Szenenkontext in einer rechten Kontextleiste anzeigen.
- Volltextsuche ueber Manuskript, Szenendaten und MVP-Projektbibel nutzen.
- KI-Seitenleiste mit drei Kernaktionen verwenden.
- Manuskript als Markdown und DOCX exportieren.

## Muss-Funktionen

- Projektverwaltung: Ein lokales Romanprojekt mit Metadaten und Projektinhalt.
- Kapitel/Szenen-Navigator: Gliederung, Reihenfolge und Auswahl der aktuellen Szene.
- Szeneneditor: Bearbeitung des Texts der aktuellen Szene.
- Szenenmetadaten: Titel, Status, Kurzbeschreibung und Kontextverknuepfungen.
- Projektbibel: Figuren, Orte, Gegenstaende und Notizen.
- Szenenkontext: Anzeige und Pflege der Eintraege, die zur aktuellen Szene gehoeren.
- Suche: Volltextsuche in Manuskript, Szenendaten und MVP-Projektbibel.
- Export: Markdown und DOCX.
- KI-Seitenleiste: kontrollierte Aktionen mit Szenen- und Projektbibel-Kontext.

## Projektbibel Im MVP

Pflichtobjekte:

- Figuren
- Orte
- Gegenstaende
- Notizen

Lore, Fraktionen und Ereignisse sind fachlich wichtig, werden aber im MVP nicht als eigene strukturierte Objekte umgesetzt. Sie koennen zunaechst als Notizen oder Notiztypen erfasst werden und werden fuer V1.x vorbereitet.

## KI-Funktionen Im MVP

Pflichtaktionen:

- Markierten Text verbessern.
- Aktuelle Szene zusammenfassen.
- Widersprueche mit Szenen- und Projektbibel-Kontext pruefen.

Implementierungsstatus Widerspruchspruefung:

- Umgesetzt als KI-Seitenleistenaktion `Widersprueche pruefen`.
- Prueft die aktuelle Szene nur gegen verknuepfte Projektbibel-Karten.
- Benoetigt Szenentext und mindestens eine verknuepfte Projektbibel-Karte.
- Speichert das Ergebnis als `AiSuggestion`; es wird nichts automatisch in Szene oder Projektbibel uebernommen.
- Widerspruchsvorschlaege koennen verworfen werden, aber nicht als Szenenzusammenfassung uebernommen werden.

Nicht im MVP, sondern V1.x:

- Dialogvarianten erzeugen.
- Figureninformationen automatisch extrahieren.
- Kapitelideen und Plotvorschlaege erzeugen.

## Export Im MVP

- Markdown-Export fuer ein offenes, gut weiterverarbeitbares Manuskript.
- DOCX-Export fuer Weitergabe, Lektorat und praktische Autoren-Workflows.

PDF, eBook-Ausgabe und Re-Import sind nicht Teil des MVP.

## Nicht Im MVP

Diese Punkte werden bewusst verschoben, damit der erste Kern stabil und nutzbar wird:

- Timeline.
- Kreativboard und freie Visualisierungsflaechen.
- Beziehungsdiagramme.
- Kartenintegration.
- Serienverwaltung.
- E-Book-Ausgabe.
- PDF-Export.
- Re-Import aus extern bearbeiteten Manuskripten.
- Agentenmodus fuer vollstaendige Manuskriptanalyse.
- Team- und Lektorfunktionen.
- Umfangreiche Plotmodell-Assistenten.
- Eigene strukturierte Objekte fuer Lore, Fraktionen und Ereignisse.

## Acceptance Criteria

- Ein Autor kann ein lokales Romanprojekt anlegen.
- Ein Autor kann Kapitel und Szenen erstellen, sortieren und bearbeiten.
- Eine Szene hat Text, Titel, Status, Kurzbeschreibung und verknuepfte Projektbibel-Eintraege.
- Figuren, Orte, Gegenstaende und Notizen koennen angelegt und mit Szenen verbunden werden.
- Die rechte Kontextleiste zeigt den Kontext der aktuellen Szene.
- Die KI-Seitenleiste kann markierten Text verbessern, eine Szene zusammenfassen und Widersprueche mit Szenen-/Projektbibel-Kontext pruefen.
- Volltextsuche findet Inhalte in Manuskript, Szenendaten und MVP-Projektbibel.
- Export erzeugt ein zusammenhaengendes Manuskript als Markdown und DOCX.

## Patchwork-Erkenntnisse Fuer Den MVP

- Der Navigator ist MVP-Kern: Autoren brauchen Kapitel und Szenen als steuerbare Romanstruktur, nicht nur eine Dateiliste.
- Der Editor soll schlicht bleiben; Formatierung und Spezialwerkzeuge duerfen den Schreibfluss nicht dominieren.
- Szenenbezogener Kontext ist wichtiger als eine isolierte Datenbank. Figuren, Orte, Gegenstaende und Notizen sollen zur aktuellen Szene sichtbar und verknuepfbar sein.
- Export ist fuer den MVP wichtig, Re-Import und eBook-Ausgabe werden bewusst verschoben.
- Einstellungen muessen im MVP minimal bleiben: Speicherort, KI-Anbieter, Exportbasis und Editorgrundverhalten.
