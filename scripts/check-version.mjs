import { readFileSync } from "node:fs";

const packageVersion = JSON.parse(readFileSync(new URL("../package.json", import.meta.url))).version;
const packageLock = JSON.parse(
  readFileSync(new URL("../package-lock.json", import.meta.url)),
);
const tauriVersion = JSON.parse(
  readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url)),
).version;
const cargoToml = readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const cargoLock = readFileSync(new URL("../src-tauri/Cargo.lock", import.meta.url), "utf8");
const cargoLockVersion = cargoLock.match(
  /\[\[package\]\]\s*name\s*=\s*"autorenprogramm"\s*version\s*=\s*"([^"]+)"/m,
)?.[1];
const packageLockVersion = packageLock.version;
const packageLockRootVersion = packageLock.packages?.[""]?.version;
const requestedTag = process.argv[2];

const versions = new Set([
  packageVersion,
  packageLockVersion,
  packageLockRootVersion,
  tauriVersion,
  cargoVersion,
  cargoLockVersion,
]);
if (versions.size !== 1 || versions.has(undefined)) {
  throw new Error(
    `Versionskonflikt: package=${packageVersion}, package-lock=${packageLockVersion}/${packageLockRootVersion}, tauri=${tauriVersion}, cargo=${cargoVersion}, cargo-lock=${cargoLockVersion}`,
  );
}

if (requestedTag && requestedTag !== `v${packageVersion}`) {
  throw new Error(`Tag ${requestedTag} passt nicht zu Version ${packageVersion}.`);
}

console.log(`Version ${packageVersion} ist konsistent${requestedTag ? ` (${requestedTag})` : ""}.`);
