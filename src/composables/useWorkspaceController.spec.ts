import { describe, expect, it } from "vitest";
import { shouldExcludeRelationshipCandidate } from "./useWorkspaceController";

describe("Beziehungsziele im Arbeitsbereich", () => {
  const character = { targetType: "character" as const, id: "character-1" };
  const worldSystem = { targetType: "world_system" as const, id: "world-1" };

  it("lässt die zuvor ausgewählte Projektbibel-Karte im Worldbuilding als Ziel zu", () => {
    expect(
      shouldExcludeRelationshipCandidate(
        character,
        "worldbuilding",
        character,
        worldSystem.id,
      ),
    ).toBe(false);
    expect(
      shouldExcludeRelationshipCandidate(
        worldSystem,
        "worldbuilding",
        character,
        worldSystem.id,
      ),
    ).toBe(true);
  });

  it("schließt in der Projektbibel nur die aktuell bearbeitete Karte aus", () => {
    expect(
      shouldExcludeRelationshipCandidate(character, "bible", character, ""),
    ).toBe(true);
    expect(
      shouldExcludeRelationshipCandidate(
        { targetType: "character", id: "character-2" },
        "bible",
        character,
        "",
      ),
    ).toBe(false);
  });
});
