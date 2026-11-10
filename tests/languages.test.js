import assert from "node:assert/strict";
import test from "node:test";
import { parseLanguages } from "../web/languages.js";

const codes = new Set(["ko", "en", "ja", "es", "fr", "de", "vi"]);

test("normalizes two-letter codes and preserves the user's order", () => {
  assert.deepEqual(parseLanguages(" KO, en\nja, es, fr ", codes), [
    "ko",
    "en",
    "ja",
    "es",
    "fr",
  ]);
});

test("rejects empty, duplicate, excessive, and invalid language lists", () => {
  for (const value of [
    ", ,",
    "en, EN",
    "1,2,3,4,5,6,7,8,9",
    "x".repeat(65),
    "한국어\u0000",
    "English",
    "eng",
    "en-US",
    "zz",
    "e1",
  ]) {
    assert.throws(() => parseLanguages(value, codes));
  }
});
