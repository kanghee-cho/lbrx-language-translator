import { parseLanguages } from "./languages.js";

const root = document.querySelector("#app");
const api = window.__TAURI__;
const tooltip = window.__LBRX_VIEW__ === "tooltip";

function message(error) {
  return error instanceof Error ? error.message : String(error);
}

function feedback(text, error = false) {
  const element = document.querySelector("#feedback");
  element.textContent = text;
  element.classList.toggle("is-error", error);
}

function readConfig() {
  return {
    targetLanguages: parseLanguages(document.querySelector("#languages").value),
    cliPath: document.querySelector("#cli-path").value.trim(),
    model: document.querySelector("#model").value.trim(),
    doubleCopyMs: Number(document.querySelector("#interval").value),
    timeoutSeconds: Number(document.querySelector("#timeout").value),
    consent: document.querySelector("#consent").checked,
  };
}

function updateChips() {
  const selected = document
    .querySelector("#languages")
    .value.split(/[,\n]/u)
    .map((language) => language.trim());
  for (const button of document.querySelectorAll(".language-chip")) {
    button.setAttribute(
      "aria-pressed",
      String(selected.includes(button.dataset.language)),
    );
  }
}

function renderRuntime(settings) {
  const status = document.querySelector("#runtime-status");
  const errors = [settings.configError, settings.listenerError].filter(Boolean);
  status.classList.toggle("is-error", errors.length > 0);
  status.textContent = errors.length
    ? errors.join("\n")
    : settings.config.consent
      ? "● 트레이에서 대기 · 단축키를 눌렀을 때만 번역합니다."
      : "○ 전송 동의 후 설정을 저장하면 번역을 시작할 수 있습니다.";
  document.querySelector(".shortcut").textContent = settings.shortcut;
  document.querySelector("#permissions").hidden = settings.platform !== "macos";
  if (settings.platform === "macos") {
    document.querySelector("#shortcut-help").textContent =
      "⌘ Command를 누른 채 C를 두 번 누릅니다. macOS의 기본 복사 키를 사용합니다. Esc로 번역창을 닫습니다.";
  }
}

async function setupSettings() {
  const settings = await api.core.invoke("get_settings");
  const config = settings.config;
  document.querySelector("#languages").value =
    config.targetLanguages.join(", ");
  document.querySelector("#cli-path").value = config.cliPath;
  document.querySelector("#model").value = config.model;
  document.querySelector("#interval").value = config.doubleCopyMs;
  document.querySelector("#timeout").value = config.timeoutSeconds;
  document.querySelector("#consent").checked = config.consent;
  renderRuntime(settings);
  updateChips();

  document.querySelector("#languages").addEventListener("input", updateChips);
  for (const button of document.querySelectorAll(".language-chip")) {
    button.addEventListener("click", () => {
      const input = document.querySelector("#languages");
      const languages = input.value
        .split(/[,\n]/u)
        .map((language) => language.trim())
        .filter(Boolean);
      const language = button.dataset.language;
      input.value = (
        languages.includes(language)
          ? languages.filter((item) => item !== language)
          : [...languages, language]
      ).join(", ");
      updateChips();
    });
  }
  document
    .querySelector("#settings-form")
    .addEventListener("submit", async (event) => {
      event.preventDefault();
      const button = document.querySelector("#save");
      button.disabled = true;
      try {
        const saved = await api.core.invoke("save_settings", {
          config: readConfig(),
        });
        renderRuntime(saved);
        feedback("저장했습니다. 창을 닫아도 트레이에서 계속 대기합니다.");
      } catch (error) {
        feedback(message(error), true);
      } finally {
        button.disabled = false;
      }
    });
  document.querySelector("#check-cli").addEventListener("click", async () => {
    const button = document.querySelector("#check-cli");
    button.disabled = true;
    feedback("CLI를 확인하고 있습니다…");
    try {
      const version = await api.core.invoke("check_cli", {
        config: readConfig(),
      });
      feedback(
        `Antigravity CLI ${version} 설치 확인. 로그인은 터미널의 agy에서 진행해주세요.`,
      );
    } catch (error) {
      feedback(message(error), true);
    } finally {
      button.disabled = false;
    }
  });
  document.querySelector("#permissions").addEventListener("click", async () => {
    try {
      await api.core.invoke("open_permissions");
      feedback(
        "손쉬운 사용에서 LBRX Translator를 허용해주세요. 입력 모니터링도 요청되면 허용한 뒤 앱을 다시 시작해주세요.",
      );
    } catch (error) {
      feedback(message(error), true);
    }
  });
  await api.event.listen("runtime-updated", async () => {
    try {
      renderRuntime(await api.core.invoke("get_settings"));
    } catch (error) {
      feedback(message(error), true);
    }
  });
}

let renderVersion = 0;
async function renderPopup() {
  const version = ++renderVersion;
  const popup = await api.core.invoke("get_popup");
  if (!popup || version !== renderVersion) return;
  const source = document.querySelector("#source");
  source.textContent = popup.source;
  source.hidden = !popup.source;
  document.querySelector("#translation-status").hidden =
    popup.status !== "loading";
  document.querySelector("#loading-message").textContent = popup.source
    ? "Gemini가 번역하고 있습니다"
    : "선택한 문장을 가져오는 중";
  const error = document.querySelector("#translation-error");
  error.textContent = popup.error ?? "";
  error.hidden = !popup.error;
  const translations = document.querySelector("#translations");
  translations.replaceChildren();
  for (const translation of popup.translations) {
    const section = document.createElement("section");
    section.className = "translation";
    const heading = document.createElement("h2");
    heading.textContent = translation.language;
    const text = document.createElement("p");
    text.dir = "auto";
    text.textContent = translation.text;
    section.append(heading, text);
    translations.append(section);
  }
  await new Promise((resolve) => requestAnimationFrame(resolve));
  if (version === renderVersion) {
    await api.core.invoke("present_popup", {
      height: document.querySelector(".popup").scrollHeight + 2,
    });
  }
}

async function setupPopup() {
  document.body.classList.add("tooltip-body");
  const render = async () => {
    try {
      await renderPopup();
    } catch (error) {
      const element = document.querySelector("#translation-error");
      element.textContent = message(error);
      element.hidden = false;
      document.querySelector("#translation-status").hidden = true;
    }
  };
  await api.event.listen("translation-updated", render);
  await render();
}

async function start() {
  if (!api) {
    throw new Error(
      "이 화면은 Tauri 데스크톱 앱에서 실행해야 합니다. npm run dev를 사용해주세요.",
    );
  }
  const template = document.querySelector(
    tooltip ? "#tooltip-template" : "#settings-template",
  );
  root.replaceChildren(template.content.cloneNode(true));
  root.setAttribute("aria-busy", "false");
  document
    .querySelector(".close-window")
    .addEventListener("click", async () => {
      try {
        await api.core.invoke("close_window");
      } catch (error) {
        if (tooltip) {
          const element = document.querySelector("#translation-error");
          element.textContent = message(error);
          element.hidden = false;
        } else {
          feedback(message(error), true);
        }
      }
    });
  if (tooltip) await setupPopup();
  else await setupSettings();
}

start().catch((error) => {
  root.setAttribute("aria-busy", "false");
  const element = document.createElement("p");
  element.className = "boot-message is-error";
  element.textContent = message(error);
  root.replaceChildren(element);
});
