import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync(new URL("../package.json", import.meta.url))).version;
const tauriVersion = JSON.parse(
  readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url)),
).version;
const cargoToml = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const requestedTag = process.argv[2];

const versions = new Set([packageVersion, tauriVersion, cargoVersion]);
if (versions.size !== 1 || versions.has(undefined)) {
  throw new Error(
    `Versionskonflikt: package=${packageVersion}, tauri=${tauriVersion}, cargo=${cargoVersion}`,
  );
}

if (requestedTag && requestedTag !== `v${packageVersion}`) {
  throw new Error(`Tag ${requestedTag} passt nicht zu Version ${packageVersion}.`);
}

console.log(`Version ${packageVersion} ist konsistent${requestedTag ? ` (${requestedTag})` : ""}.`);
