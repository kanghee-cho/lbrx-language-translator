import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const checker = fileURLToPath(
  new URL("../scripts/check-version.mjs", import.meta.url),
);

function fixture(t, overrides = {}) {
  const root = mkdtempSync(join(tmpdir(), "lbrx-version-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "src-tauri"));
  const files = {
    "package.json": '{"version":"1.0.0"}',
    "package-lock.json":
      '{"version":"1.0.0","packages":{"":{"version":"1.0.0"}}}',
    "src-tauri/tauri.conf.json": '{"version":"1.0.0"}',
    "src-tauri/Cargo.toml":
      '[package]\nname = "lbrx-language-translator"\nversion = "1.0.0"\n\n[dependencies]\n',
    "src-tauri/Cargo.lock":
      'version = 4\n\n[[package]]\nname = "lbrx-language-translator"\nversion = "1.0.0"\n\n[[package]]\nname = "serde"\nversion = "1.0.228"\n',
    ...overrides,
  };
  for (const [file, content] of Object.entries(files)) {
    writeFileSync(join(root, file), content);
  }
  return root;
}

test("accepts synchronized manifest versions and a matching release tag", (t) => {
  const result = spawnSync(process.execPath, [checker, "v1.0.0"], {
    cwd: fixture(t),
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
});

test("rejects version drift in manifests, lockfiles, and release tags", (t) => {
  for (const [file, content] of [
    [
      "package-lock.json",
      '{"version":"0.1.0","packages":{"":{"version":"1.0.0"}}}',
    ],
    [
      "package-lock.json",
      '{"version":"1.0.0","packages":{"":{"version":"0.1.0"}}}',
    ],
    ["src-tauri/tauri.conf.json", '{"version":"0.1.0"}'],
    ["src-tauri/Cargo.toml", '[package]\nversion = "0.1.0"\n'],
    [
      "src-tauri/Cargo.lock",
      '[[package]]\nname = "lbrx-language-translator"\nversion = "0.1.0"\n',
    ],
  ]) {
    const result = spawnSync(process.execPath, [checker], {
      cwd: fixture(t, { [file]: content }),
      encoding: "utf8",
    });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /must match package\.json/u);
  }
  const result = spawnSync(process.execPath, [checker, "v1.0.1"], {
    cwd: fixture(t),
    encoding: "utf8",
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Release tag must match/u);
});
