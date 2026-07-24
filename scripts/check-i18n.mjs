import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "@vue/compiler-sfc";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (relativePath) => fs
  .readFileSync(path.join(root, relativePath), "utf8")
  .replace(/\r\n?/g, "\n");

const source = read("src/i18n.ts");
const deSource = source.match(/const de = (\{[\s\S]*?\n\});\n\nconst en =/)?.[1];
const enSource = source.match(/const en = (\{[\s\S]*?\n\});\n\nexport const i18n/)?.[1];
if (!deSource || !enSource) throw new Error("Die Übersetzungskataloge konnten nicht gelesen werden.");

const evaluate = (value) => Function(`"use strict"; return (${value});`)();
const de = evaluate(deSource);
const en = evaluate(enSource);
const keys = (value, prefix = "") => Object.entries(value).flatMap(([key, child]) => {
  const current = prefix ? `${prefix}.${key}` : key;
  return child && typeof child === "object" ? keys(child, current) : [current];
}).sort();

const deKeys = keys(de);
const enKeys = keys(en);
if (JSON.stringify(deKeys) !== JSON.stringify(enKeys)) {
  const missingEn = deKeys.filter((key) => !enKeys.includes(key));
  const missingDe = enKeys.filter((key) => !deKeys.includes(key));
  throw new Error(`Katalogschlüssel unterscheiden sich. Fehlt EN: ${missingEn.join(", ")}; fehlt DE: ${missingDe.join(", ")}`);
}

function filesBelow(directory, extension) {
  return fs.readdirSync(path.join(root, directory), { withFileTypes: true }).flatMap((entry) => {
    const relativePath = path.join(directory, entry.name);
    return entry.isDirectory() ? filesBelow(relativePath, extension) : entry.name.endsWith(extension) ? [relativePath] : [];
  });
}

const vueFiles = filesBelow("src", ".vue");
const vueSources = vueFiles.map((file) => ({ file, source: read(file) }));
const generated = JSON.parse(read("src/locales/generated-ui.json"));
const usedGenerated = vueSources.flatMap(({ source: vueSource }) =>
  [...vueSource.matchAll(/\bui\("([^"]+)"\)/g)].map((match) => match[1]),
);
const missingGenerated = usedGenerated.filter((key) => !(key in generated));
if (missingGenerated.length) throw new Error(`Fehlende UI-Übersetzungen: ${[...new Set(missingGenerated)].join(", ")}`);
const emptyGenerated = Object.entries(generated).filter(([, value]) => typeof value !== "string" || !value.trim()).map(([key]) => key);
if (emptyGenerated.length) throw new Error(`Leere UI-Übersetzungen: ${emptyGenerated.join(", ")}`);

const ignoredVisibleText = new Set([
  "Autorenprogramm",
  "http://localhost:11434/v1",
  "px",
  "×",
  "·",
  "Markdown / DOCX",
]);
const hardcodedVisible = [];
function inspectTemplateNode(node, file) {
  if (node.type === 2) {
    const value = node.content.replace(/\s+/g, " ").trim();
    if (/[A-Za-zÄÖÜäöüß]/.test(value) && !ignoredVisibleText.has(value)) {
      hardcodedVisible.push(`${file}: ${value}`);
    }
  }
  if (node.type === 1) {
    for (const prop of node.props ?? []) {
      if (prop.type === 6 && ["placeholder", "title", "aria-label"].includes(prop.name) && /[A-Za-zÄÖÜäöüß]/.test(prop.value?.content ?? "")) {
        hardcodedVisible.push(`${file}: ${prop.name}=${prop.value.content}`);
      }
    }
  }
  for (const child of node.children ?? []) inspectTemplateNode(child, file);
}
for (const { file, source: vueSource } of vueSources) {
  const result = parse(vueSource, { filename: file });
  if (result.errors.length) throw new Error(`Vue-Template konnte nicht geprüft werden: ${file}`);
  if (result.descriptor.template?.ast) inspectTemplateNode(result.descriptor.template.ast, file);
}
if (hardcodedVisible.length) throw new Error(`Nicht katalogisierte sichtbare Texte: ${hardcodedVisible.join(" | ")}`);

const rustFiles = filesBelow("src-tauri/src", ".rs");
const rustSource = rustFiles.map(read).join("\n");
const errorCodes = new Set(
  [...rustSource.matchAll(/(?:AppError|Self)::new\(\s*"([a-z][a-z0-9_]+)"/g)].map((match) => match[1]),
);
for (const match of rustSource.matchAll(/require_changed\(/g)) {
  const call = rustSource.slice(match.index, rustSource.indexOf(")", match.index) + 1);
  const code = call.match(/"([a-z][a-z0-9_]+)"/)?.[1];
  if (code) errorCodes.add(code);
}
const frontendSource = [source, read("src/errors.ts"), read("src/api/tauri.ts"), ...filesBelow("src/stores", ".ts").map(read), ...vueSources.map((item) => item.source)].join("\n");
for (const pattern of [
  /\buiError\(\s*"([a-z][a-z0-9_]+)"/g,
  /\b(?:normalizeUiError|setError|setCopyError)\([^,\n]+,\s*"([a-z][a-z0-9_]+)"/g,
  /\bcode:\s*"([a-z][a-z0-9_]+)"/g,
]) {
  for (const match of frontendSource.matchAll(pattern)) errorCodes.add(match[1]);
}
const missingErrorTranslations = [...errorCodes].filter((code) => !(code in de.errors)).sort();
if (missingErrorTranslations.length) throw new Error(`Fehlende Fehlerübersetzungen: ${missingErrorTranslations.join(", ")}`);

const mojibakeFiles = ["src/i18n.ts", "src/locales/generated-ui.json", ...vueFiles];
const mojibake = mojibakeFiles.filter((file) => /Ã|Â|�/.test(read(file)));
if (mojibake.length) throw new Error(`Beschädigte UTF-8-Zeichen gefunden: ${mojibake.join(", ")}`);

console.log(`${deKeys.length} identische Katalogschlüssel, ${Object.keys(generated).length} UI-Texte und ${errorCodes.size} Fehlercodes geprüft.`);
