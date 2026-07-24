import { describe, expect, it } from "vitest";
import type { BibleRelationshipSnapshot } from "../api/projectBible";
import { i18n, setLocale } from "../i18n";
import {
  isRelationshipTypeAllowed,
  relationshipDefinitions,
  relationshipDisplayLabel,
} from "./relationships";

describe("Beziehungstypen", () => {
  it("wendet die fachliche Typmatrix an", () => {
    expect(isRelationshipTypeAllowed("character", "character", "family")).toBe(true);
    expect(isRelationshipTypeAllowed("faction", "character", "family")).toBe(false);
    expect(isRelationshipTypeAllowed("character", "faction", "member_of")).toBe(true);
    expect(isRelationshipTypeAllowed("character", "location", "member_of")).toBe(false);
    expect(isRelationshipTypeAllowed("item", "character", "created_by")).toBe(true);
    expect(isRelationshipTypeAllowed("character", "item", "created_by")).toBe(false);
    expect(isRelationshipTypeAllowed("world_system", "character", "affects")).toBe(true);
    expect(isRelationshipTypeAllowed("character", "location", "affects")).toBe(false);
    expect(isRelationshipTypeAllowed("event", "event", "follows")).toBe(true);
    expect(isRelationshipTypeAllowed("event", "character", "follows")).toBe(false);
  });

  it("besitzt für jeden stabilen Typ eindeutige Vorwärts- und Gegenlabels", () => {
    const ids = relationshipDefinitions.map((definition) => definition.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const definition of relationshipDefinitions) {
      expect(i18n.global.te(definition.labelKey, "de")).toBe(true);
      expect(i18n.global.te(definition.reverseKey, "de")).toBe(true);
      expect(i18n.global.te(definition.labelKey, "en")).toBe(true);
      expect(i18n.global.te(definition.reverseKey, "en")).toBe(true);
    }
  });

  it("zeigt abhängig von der Blickrichtung das richtige Label in beiden Sprachen", () => {
    const relationship = {
      relationshipType: "uses_system",
      sourceType: "character",
      sourceId: "character-1",
    } satisfies Pick<BibleRelationshipSnapshot, "relationshipType" | "sourceType" | "sourceId">;
    const translate = (key: string) => i18n.global.t(key);

    setLocale("de");
    expect(relationshipDisplayLabel(relationship, "character", "character-1", translate)).toBe("nutzt System");
    expect(relationshipDisplayLabel(relationship, "world_system", "system-1", translate)).toBe("wird genutzt von");
    setLocale("en");
    expect(relationshipDisplayLabel(relationship, "character", "character-1", translate)).toBe("uses system");
    expect(relationshipDisplayLabel(relationship, "world_system", "system-1", translate)).toBe("is used by");
  });
});
