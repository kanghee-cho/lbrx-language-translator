use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

pub const MAX_SOURCE_CHARS: usize = 12_000;

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
            target_languages: vec!["한국어".into()],
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
            let language = language.trim();
            if language.is_empty()
                || language.chars().count() > 64
                || language.chars().any(char::is_control)
            {
                return Err("언어 이름은 줄바꿈 없이 1~64자로 입력해주세요.".into());
            }
            if languages
                .iter()
                .any(|item| item.to_lowercase() == language.to_lowercase())
            {
                return Err(format!("번역 언어가 중복되었습니다: {language}"));
            }
            languages.push(language.to_owned());
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
        Ok(bytes) => serde_json::from_slice::<Config>(&bytes)
            .map_err(|error| format!("설정 파일을 읽을 수 없습니다: {error}"))?
            .validated(),
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
            target_languages: vec![" 한국어 ".into(), "English".into(), "日本語".into()],
            ..Config::default()
        }
        .validated()
        .unwrap();
        assert_eq!(config.target_languages, ["한국어", "English", "日本語"]);
        assert!(Config {
            target_languages: vec!["English".into(), " english ".into()],
            ..config
        }
        .validated()
        .is_err());
    }

    #[test]
    fn rejects_invalid_configuration() {
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
    fn persists_and_replaces_configuration_without_losing_unicode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = Config::default();
        save(&path, &config).unwrap();
        config.target_languages.push("日本語".into());
        save(&path, &config).unwrap();
        assert_eq!(
            load(&path).unwrap().target_languages,
            config.target_languages
        );
        std::fs::write(&path, "broken").unwrap();
        assert!(load(&path).is_err());
    }
}
