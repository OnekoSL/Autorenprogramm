import type { CommandError } from "./project";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

export function requireTauriRuntime() {
  if (isTauriRuntime()) {
    return;
  }

  throw {
    code: "tauri_unavailable",
    message:
      "Das Desktop-Backend ist nicht verbunden. Bitte starte die App mit `npm.cmd run tauri dev`; im normalen Browser koennen keine lokalen Projektdateien angelegt oder geoeffnet werden.",
  } satisfies CommandError;
}
