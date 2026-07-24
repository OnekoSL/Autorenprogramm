# Projektgebundene Mehrsprachigkeit

## Verbindliche Produktentscheidung

Ein Projekt verwendet dauerhaft genau eine Sprache: Deutsch (`de`, Anzeigeformat `de-DE`) oder britisches Englisch (`en`, Anzeigeformat `en-GB`). Die Sprache wird beim Anlegen gewählt und kann danach nicht geändert werden. Bestehende Projekte werden als deutsch behandelt. Autorentexte und bereits angewandte Vorlagen werden niemals automatisch übersetzt.

Vor dem Öffnen eines Projekts startet die Anwendung deutsch. Die Auswahl im Anlegedialog dient zugleich als Vorschau. Beim Öffnen übernimmt die gesamte Oberfläche die Projektsprache; beim Schließen kehrt sie zu Deutsch zurück.

## Technisches Modell

Schema 16 normalisiert `default_language` auf `de` oder `en`. Gleichzeitig werden die gespeicherten Szenenstatus auf die sprachneutralen IDs `draft`, `revision` und `complete` migriert. Kategorien, Beziehungstypen, Status und Vorlagen-IDs bleiben ebenfalls sprachneutral; nur ihre Anzeige wird übersetzt.

Das Frontend verwendet `vue-i18n` und die Locale `de-DE` beziehungsweise `en-GB` für Texte, Zahlen, Datum, Uhrzeit und Sortierung. Das HTML-Attribut `lang` folgt der aktiven Locale. Gefangene Frontend-, Dialog-, Tauri- und Backendfehler werden intern als `UiError { code, detail? }` gehalten. Übersetzt wird erst beim Rendern, damit sich auch eine bereits sichtbare Fehlermeldung bei einem Sprachwechsel aktualisiert. Die normale Anzeige enthält die lokalisierte Meldung und den stabilen Code in Klammern; technische Details bleiben reine Diagnoseinformation.

## Standardinhalte und Vorlagen

Neue Projekte erhalten lokalisierte Namen für den ersten Bereich, das erste Kapitel und die erste Szene. Worldbuilding- und Story-Strukturvorlagen behalten in beiden Sprachen identische IDs. Eine neu angewandte Vorlage wird in der Projektsprache kopiert; bestehende Projektkopien bleiben unverändert. Kann eine lokalisierte Ressource nicht gelesen werden, bleibt mindestens die passende allgemeine Fallbackvorlage verfügbar.

## KI und Export

Alle KI-Aktionen weisen den Provider standardmäßig zur Projektsprache an. Für englische Projekte gilt britisches Englisch. Verlangt der Bearbeitungsauftrag ausdrücklich eine andere Sprache, hat er Vorrang. Gespeicherte KI-Ergebnisse und Autorentexte werden nicht nachträglich übersetzt. Beim Übernehmen einer KI-Antwort werden bekannte deutsche und englische Abschnittsüberschriften entfernt, ohne den eigentlichen Inhalt zu übersetzen.

Beim Export, bei Sicherungen und bei Fallbackpfaden werden ausschließlich programmgenerierte Bezeichnungen und Standarddateinamen lokalisiert. Dazu gehören `Manuskript`/`Manuscript`, `Projekt`/`Project` und `Zwischenstand`/`Snapshot`. Der Projektname und vorhandene Nutzerinhalte bleiben unverändert.

## Qualitätsregeln

- Deutsch und Englisch besitzen dieselben Übersetzungsschlüssel.
- Sichtbare Programmtexte werden nicht aus übersetzbaren Satzfragmenten zusammengesetzt.
- Zählbare Begriffe verwenden die Pluralregeln der Locale.
- Neue öffentliche Sprach- und Szenenstatuswerte werden in TypeScript typsicher geführt.
- Migration, Projekterstellung, Sprachsperre, Vorlagenfallback und KI-Sprachanweisung werden durch Regressionstests abgesichert.
- `npm run check:i18n` prüft beide Kataloge, alle Vue-Templates einschließlich Textfragmenten und statischen sichtbaren Attributen, alle verwendeten `ui()`-Texte, bekannte Backend- und Frontendfehlercodes sowie beschädigte UTF-8-Zeichen. CI und Release führen diese Prüfung vor dem Frontend-Build aus.
- Die repositoryweite Arbeitsanweisung `AGENTS.md` macht diese Prüfung für jede zukünftige Funktionsänderung verbindlich. Mehrsprachigkeit ist kein nachgelagerter Übersetzungsschritt, sondern Teil der Definition of Done.
- Deutsche Programm- und Dokumentationstexte verwenden korrektes UTF-8 mit Umlauten und `ß`; Umschreibungen wie `ae`, `oe`, `ue` oder ein unzutreffendes `ss` sind keine zulässige Lokalisierung.

Weitere Sprachen können später ergänzt werden. Eine nachträgliche Änderung der Projektsprache oder automatische Übersetzung von Nutzerdaten bleibt eine eigene, ausdrücklich zu entscheidende Funktion.

## Umgesetzter Stand

Die Oberfläche verwendet vollständige deutsche und britisch-englische Kataloge einschließlich Fachlabels, Dialogen, Leerzuständen, Status- und Fehlermeldungen. Beziehungstypen und Gegenlabels werden aus stabilen IDs übersetzt. Zähler verwenden Locale-Pluralisierung; Datum, Zahlen, Suche und Sortierung folgen `de-DE` beziehungsweise `en-GB`.

Story-Struktur- und Worldbuilding-Ressourcen liegen in beiden Sprachen mit identischen stabilen IDs vor. Neue Vorlagenkopien werden anhand der Projektsprache gewählt. Sämtliche KI-Aktionen erhalten dieselbe projektgebundene Sprachvorgabe mit Vorrang für einen ausdrücklich anderssprachigen Auftrag. Der automatische Katalog-, Template-, UTF-8- und Fehlercodetest verhindert neue unübersetzte Oberflächentexte und Fehlercodes.
