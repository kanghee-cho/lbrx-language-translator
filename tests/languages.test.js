import assert from "node:assert/strict";
import test from "node:test";
import { parseLanguages } from "../web/languages.js";

test("parses custom languages and preserves the user's order", () => {
  assert.deepEqual(parseLanguages(" 한국어, English\n日本語, Español "), [
    "한국어",
    "English",
    "日本語",
    "Español",
  ]);
});

test("rejects empty, duplicate, excessive, and invalid language lists", () => {
  for (const value of [
    ", ,",
    "English, english",
    "1,2,3,4,5,6,7,8,9",
    "x".repeat(65),
    "한국어\u0000",
  ]) {
    assert.throws(() => parseLanguages(value));
  }
});
