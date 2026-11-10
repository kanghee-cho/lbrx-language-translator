import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (file) => readFileSync(file, "utf8");
const json = (file) => JSON.parse(read(file));
const cargoVersion = (section) =>
  section?.match(/^version\s*=\s*"([^"]+)"/mu)?.[1];
const manifest = json("package.json");
const lock = json("package-lock.json");
const cargo = read("src-tauri/Cargo.toml")
  .split(/(?=^\[)/mu)
  .find((section) => section.startsWith("[package]"));
const cargoLock = read("src-tauri/Cargo.lock")
  .split(/(?=^\[)/mu)
  .find((section) => /^name\s*=\s*"lbrx-language-translator"$/mu.test(section));
const version = manifest.version;
assert.match(version, /^\d+\.\d+\.\d+$/u, "Expected a stable release version");
for (const [file, value] of Object.entries({
  "package-lock.json": lock.version,
  "package-lock.json root package": lock.packages?.[""]?.version,
  "src-tauri/tauri.conf.json": json("src-tauri/tauri.conf.json").version,
  "src-tauri/Cargo.toml": cargoVersion(cargo),
  "src-tauri/Cargo.lock": cargoVersion(cargoLock),
})) {
  assert.equal(value, version, `${file} must match package.json (${version})`);
}
const tag = process.argv[2];
if (tag !== undefined) {
  assert.equal(tag, `v${version}`, "Release tag must match the manifests");
}
console.log(`Version ${version}: all manifests${tag ? " and tag" : ""} match.`);
