import { afterEach } from "vitest";
import { setLocale } from "../i18n";

afterEach(() => {
  setLocale("de");
  delete window.__TAURI_INTERNALS__;
  document.body.innerHTML = "";
});
