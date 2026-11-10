use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

pub const MAX_SOURCE_CHARS: usize = 12_000;
pub const LANGUAGE_CODES: &str = "aa ab ae af ak am an ar as av ay az ba be bg bi bm bn bo br bs ca ce ch co cr cs cu cv cy da de dv dz ee el en eo es et eu fa ff fi fj fo fr fy ga gd gl gn gu gv ha he hi ho hr ht hu hy hz ia id ie ig ii ik io is it iu ja jv ka kg ki kj kk kl km kn ko kr ks ku kv kw ky la lb lg li ln lo lt lu lv mg mh mi mk ml mn mr ms mt my na nb nd ne ng nl nn no nr nv ny oc oj om or os pa pi pl ps pt qu rm rn ro ru rw sa sc sd se sg si sk sl sm sn so sq sr ss st su sv sw ta te tg th ti tk tl tn to tr ts tt tw ty ug uk ur uz ve vi vo wa wo xh yi yo za zh zu";

pub fn is_language_code(code: &str) -> bool {
    LANGUAGE_CODES
        .split_ascii_whitespace()
        .any(|item| item == code)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub target_languages: Vec<String>,
    pub cli_path: String,
    pub model: String,
    pub double_copy_ms: u64,
    pub timeout_seconds: u64,
    pub consent: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target_languages: vec!["ko".into()],
            cli_path: String::new(),
            model: "gemini-3.8-flash-low".into(),
            double_copy_ms: 500,
            timeout_seconds: 90,
            consent: false,
        }
    }
}

impl Config {
    pub fn validated(mut self) -> Result<Self, String> {
        if self.target_languages.is_empty() || self.target_languages.len() > 8 {
            return Err("번역 언어를 1~8개 지정해주세요.".into());
        }
        let mut languages = Vec::<String>::new();
        for language in self.target_languages {
            let language = language.trim().to_ascii_lowercase();
            if !is_language_code(&language) {
                return Err(format!(
                    "올바른 ISO 639-1 두 자리 언어 코드를 입력해주세요: {language}"
                ));
            }
            if languages.contains(&language) {
                return Err(format!("번역 언어가 중복되었습니다: {language}"));
            }
            languages.push(language);
        }
        self.target_languages = languages;
        self.cli_path = self.cli_path.trim().to_owned();
        self.model = self.model.trim().to_owned();
        if !self.model.starts_with("gemini-")
            || self.model.len() > 128
            || self.model.starts_with('-')
            || self.model.chars().any(char::is_whitespace)
        {
            return Err(
                "agy models에 표시되는 gemini-로 시작하는 모델 이름을 입력해주세요.".into(),
            );
        }
        if !(200..=1_000).contains(&self.double_copy_ms) {
            return Err("단축키 간격은 200~1,000ms로 지정해주세요.".into());
        }
        if !(15..=180).contains(&self.timeout_seconds) {
            return Err("응답 제한 시간은 15~180초로 지정해주세요.".into());
        }
        Ok(self)
    }
}

pub fn load(path: &Path) -> Result<Config, String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut config: Config = serde_json::from_slice(&bytes)
                .map_err(|error| format!("설정 파일을 읽을 수 없습니다: {error}"))?;
            let mut languages = Vec::new();
            for language in &config.target_languages {
                let normalized = language.trim().to_lowercase();
                let code = match normalized.as_str() {
                    "한국어" | "한글" | "korean" => "ko",
                    "english" | "영어" => "en",
                    "日本語" | "일본어" | "japanese" => "ja",
                    "español" | "spanish" | "스페인어" => "es",
                    "简体中文" | "繁體中文" | "中文" | "chinese" | "중국어" => "zh",
                    _ => &normalized,
                }
                .to_owned();
                languages.push(code);
            }
            config.target_languages = languages;
            config.validated()
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(format!("설정 파일을 열 수 없습니다: {error}")),
    }
}

pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    let parent = path.parent().ok_or("설정 경로가 올바르지 않습니다.")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    temporary
        .write_all(&bytes)
        .map_err(|error| error.to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    temporary
        .persist(path)
        .map_err(|error| format!("설정 저장 실패: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_multiple_languages_and_rejects_duplicates() {
        let config = Config {
            target_languages: vec![" KO ".into(), "en".into(), "ja".into()],
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(config.target_languages, ["ko", "en", "ja"]);
        assert!(Config {
            target_languages: vec!["en".into(), " EN ".into()],
            ..config
        }
        .validated()
        .is_err());
    }

    #[test]
    fn rejects_invalid_configuration() {
        for language in ["English", "zz", "eng", "en-US", "e1"] {
            assert!(Config {
                target_languages: vec![language.into()],
                ..Config::default()
            }
            .validated()
            .is_err());
        }
        assert!(Config {
            target_languages: vec![],
            ..Config::default()
        }
        .validated()
        .is_err());
        assert!(Config {
            double_copy_ms: 0,
            ..Config::default()
        }
        .validated()
        .is_err());
        assert!(Config {
            timeout_seconds: 0,
            ..Config::default()
        }
        .validated()
        .is_err());
        assert!(Config {
            model: "--yolo".into(),
            ..Config::default()
        }
        .validated()
        .is_err());
        assert!(Config {
            model: "claude-sonnet-5".into(),
            ..Config::default()
        }
        .validated()
        .is_err());
    }

    #[test]
    fn persists_and_replaces_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = Config::default();
        save(&path, &config).unwrap();
        config.target_languages.push("ja".into());
        save(&path, &config).unwrap();
        assert_eq!(
            load(&path).unwrap().target_languages,
            config.target_languages
        );
        std::fs::write(&path, "broken").unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn migrates_old_presets_without_overwriting_settings_or_losing_consent() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        let old = r#"{"targetLanguages":["한국어","English","日本語","Español","简体中文"],"consent":true}"#;
        std::fs::write(&path, old).unwrap();
        let config = load(&path).unwrap();
        assert_eq!(config.target_languages, ["ko", "en", "ja", "es", "zh"]);
        assert!(config.consent);
        assert_eq!(std::fs::read_to_string(path).unwrap(), old);
    }
}
