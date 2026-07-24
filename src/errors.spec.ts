import { describe, expect, it } from "vitest";
import { formatUiError, normalizeUiError, uiError } from "./errors";
import { setLocale } from "./i18n";

describe("UiError", () => {
  it("übernimmt Backendcode und technische Diagnose getrennt", () => {
    expect(normalizeUiError({ code: "scene_missing", message: "SQL row not found" })).toEqual({
      code: "scene_missing",
      detail: "SQL row not found",
    });
  });

  it("normalisiert JavaScript- und Stringfehler mit einem stabilen Fallbackcode", () => {
    expect(normalizeUiError(new Error("Dialog kaputt"), "file_dialog_failed")).toEqual({
      code: "file_dialog_failed",
      detail: "Dialog kaputt",
    });
    expect(normalizeUiError("Netzwerkdetail")).toEqual({
      code: "unexpected_error",
      detail: "Netzwerkdetail",
    });
  });

  it("übersetzt erst beim Rendern und zeigt keine Rohdetails", () => {
    const error = uiError("scene_missing", "SQL row not found");
    setLocale("de");
    expect(formatUiError(error)).toBe("Die Szene wurde nicht gefunden. (scene_missing)");
    setLocale("en");
    expect(formatUiError(error)).toBe("The scene could not be found. (scene_missing)");
    expect(formatUiError(error)).not.toContain("SQL");
  });

  it("verwendet für unbekannte Codes die lokalisierte Standardmeldung", () => {
    setLocale("en");
    expect(formatUiError(uiError("mystery_failure", "secret"))).toBe(
      "The action failed unexpectedly. (mystery_failure)",
    );
  });
});
