const displayNames = new Intl.DisplayNames(["ko"], { type: "language" });
const englishNames = new Intl.DisplayNames(["en"], { type: "language" });

export function languageLabel(code) {
  return `${languageName(code)} (${code})`;
}

export function languageName(code) {
  return displayNames.of(code);
}

export function languageEnglishName(code) {
  return englishNames.of(code);
}

export function parseLanguages(value, allowedCodes) {
  const languages = value
    .split(/[,\n]/u)
    .map((language) => language.trim().toLowerCase())
    .filter(Boolean);
  if (languages.length < 1 || languages.length > 8) {
    throw new Error("번역 언어를 1~8개 지정해주세요.");
  }
  const seen = new Set();
  for (const language of languages) {
    if (!/^[a-z]{2}$/u.test(language) || !allowedCodes.has(language)) {
      throw new Error(`올바른 두 자리 언어 코드를 입력해주세요: ${language}`);
    }
    const key = language.toLowerCase();
    if (seen.has(key)) {
      throw new Error(`번역 언어가 중복되었습니다: ${language}`);
    }
    seen.add(key);
  }
  return languages;
}
