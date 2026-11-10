export function parseLanguages(value) {
  const languages = value
    .split(/[,\n]/u)
    .map((language) => language.trim())
    .filter(Boolean);
  if (languages.length < 1 || languages.length > 8) {
    throw new Error("번역 언어를 1~8개 지정해주세요.");
  }
  const seen = new Set();
  for (const language of languages) {
    if ([...language].length > 64 || /\p{Cc}/u.test(language)) {
      throw new Error("언어 이름은 줄바꿈 없이 1~64자로 입력해주세요.");
    }
    const key = language.toLowerCase();
    if (seen.has(key)) {
      throw new Error(`번역 언어가 중복되었습니다: ${language}`);
    }
    seen.add(key);
  }
  return languages;
}
