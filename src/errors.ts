import { i18n } from "./i18n";

export interface CommandError {
  code: string;
  message: string;
}

export interface UiError {
  code: string;
  detail?: string;
}

function errorDetail(error: unknown): string | undefined {
  if (typeof error === "object" && error && "detail" in error) {
    const detail = String((error as { detail: unknown }).detail ?? "").trim();
    if (detail) return detail;
  }

  if (typeof error === "object" && error && "message" in error) {
    const message = String((error as { message: unknown }).message).trim();
    return message || undefined;
  }

  if (typeof error === "string") {
    const message = error.trim();
    return message || undefined;
  }

  return undefined;
}

export function normalizeUiError(
  error: unknown,
  fallbackCode = "unexpected_error",
): UiError {
  const suppliedCode =
    typeof error === "object" && error && "code" in error
      ? String((error as { code: unknown }).code).trim()
      : "";

  return {
    code: suppliedCode || fallbackCode,
    detail: errorDetail(error),
  };
}

export function uiError(code: string, detail?: string): UiError {
  return { code, detail: detail?.trim() || undefined };
}

export function formatUiError(error: UiError | null | undefined): string {
  if (!error) return "";

  const key = `errors.${error.code}`;
  const translated = i18n.global.t(key);
  const message = translated === key ? i18n.global.t("errors.unexpected_error") : translated;
  return `${message} (${error.code})`;
}
