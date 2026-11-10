import assert from "node:assert/strict";
import test from "node:test";

test("presents a hidden popup without waiting for an animation frame", async (t) => {
  const globals = ["document", "window", "requestAnimationFrame"];
  const originals = globals.map((key) =>
    Object.getOwnPropertyDescriptor(globalThis, key),
  );
  t.after(() => {
    globals.forEach((key, index) => {
      if (originals[index]) {
        Object.defineProperty(globalThis, key, originals[index]);
      } else {
        delete globalThis[key];
      }
    });
  });
  const elements = new Map();
  const element = () => ({
    classList: { add() {} },
    setAttribute() {},
    addEventListener() {},
    replaceChildren() {},
    content: { cloneNode: () => ({}) },
    scrollHeight: 320,
  });
  globalThis.document = {
    body: element(),
    querySelector(selector) {
      if (!elements.has(selector)) elements.set(selector, element());
      return elements.get(selector);
    },
  };
  const presentations = [];
  globalThis.window = {
    __LBRX_VIEW__: "tooltip",
    __TAURI__: {
      event: { listen: async () => () => {} },
      core: {
        async invoke(command, args) {
          if (command === "get_popup") {
            return {
              status: "loading",
              source: "",
              error: null,
              translations: [],
            };
          }
          assert.equal(command, "present_popup");
          presentations.push(args);
        },
      },
    },
  };
  globalThis.requestAnimationFrame = () => {};
  await import("../web/app.js");
  await new Promise(setImmediate);
  assert.deepEqual(presentations, [{ height: 322 }]);
});
