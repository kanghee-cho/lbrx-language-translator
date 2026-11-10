# LBRX Translator

선택한 문장을 여러 언어로 번역해 커서 옆의 작은 창에 보여주는 Windows/macOS 트레이 앱입니다.

**Rust + Tauri 2 + 순수 HTML/CSS/JavaScript.** React, 프런트엔드 번들러, 상주 Node 서버는 사용하지 않습니다. 대기 중에는 OS 입력 이벤트만 수신하며 클립보드를 주기적으로 조회하지 않습니다. 설정창과 번역창은 필요할 때 생성하고 닫으면 WebView를 제거합니다. 번역 요청 중에만 Go 기반 공식 **Antigravity CLI (`agy`)**를 실행해 Gemini 모델을 사용합니다. 완성된 앱 실행에는 Node.js가 필요하지 않습니다.

## 다운로드 · v1.0.1

[GitHub Releases](https://github.com/kanghee-cho/lbrx-language-translator/releases)에서 Windows x64용 포터블 ZIP·설치 EXE와 macOS 12 이상용 Universal DMG·앱 ZIP을 내려받을 수 있습니다. macOS Universal 앱은 Apple Silicon과 Intel을 모두 지원합니다. 실행 전 아래의 Antigravity CLI 설치와 Google 로그인이 필요합니다.

Windows 버전은 미서명이며 macOS는 ad-hoc 서명만 적용되어 Developer ID 서명·공증은 되어 있지 않습니다. SmartScreen 또는 Gatekeeper가 경고할 수 있으므로 신뢰하는 배포본인지 확인한 뒤 운영체제의 허용 절차를 사용하세요. 체크섬은 릴리스의 `SHA256SUMS.txt`에서 확인할 수 있습니다.

## 실행 준비

1. [공식 설치 안내](https://github.com/google-antigravity/antigravity-cli#installation)에 따라 Antigravity CLI **1.5 이상**을 설치합니다. 이미 설치했다면 `agy --version`으로 확인하세요.

   ```powershell
   agy --version
   ```

2. 터미널에서 `agy`를 실행해 Google AI Pro/Ultra 구독 계정으로 로그인하고 종료합니다. 앱은 CLI의 기존 로그인 정보를 사용합니다. 로그인 토큰을 별도로 추출하거나 저장하지 않습니다.
3. 소스에서 개발·빌드하려면 Node.js 22 이상, Rust stable과 [Tauri 개발 환경](https://v2.tauri.app/start/prerequisites/)을 준비합니다. Windows는 MSVC C++ Build Tools와 WebView2, macOS는 Xcode Command Line Tools가 필요합니다.
4. 프로젝트 폴더에서 실행합니다.

   ```powershell
   npm ci
   npm run dev
   ```

처음 실행하면 설정창이 열립니다. 번역 언어를 지정하고 전송 동의 후 저장하세요. 다음 실행부터는 트레이에서 대기합니다. 앱의 자동 시작 등록은 하지 않습니다.

## 사용법

| 플랫폼 | 단축키 |
| --- | --- |
| Windows | Ctrl을 누른 채 C를 빠르게 두 번 |
| macOS | ⌘ Command를 누른 채 C를 빠르게 두 번 |

기본 간격은 500ms이며 설정에서 변경할 수 있습니다. 키를 길게 누르는 자동 반복은 번역을 실행하지 않습니다. 일반 복사는 차단하지 않습니다. 실제 복사로 클립보드가 갱신된 경우에만 텍스트를 읽습니다. 복사 기능을 지원하지 않는 앱, 보안 입력란, 관리자 권한이 다른 앱에서는 사용할 수 없을 수 있습니다. 단축키를 쓰면 일반 복사와 마찬가지로 클립보드 내용이 선택 문장으로 바뀝니다.

번역은 한 번에 12,000자, 대상 언어는 최대 8개입니다. **ISO 639-1 두 자리 코드**를 쉼표로 입력합니다. 예: `ko, en, ja, es, fr, de, vi`. 설정의 **두 자리 언어 코드 보기**를 누르면 코드·한국어/영어 이름을 검색할 수 있는 팝업이 열리며, 코드를 눌러 추가·해제할 수 있습니다. 등록되지 않은 코드는 저장할 수 없습니다. 목록은 [미국 의회도서관의 ISO 언어 코드 레지스트리](https://www.loc.gov/standards/iso639-2/php/code_list.php)를 기준으로 합니다.

하나의 Gemini 요청에서 원문의 주 언어를 감지하고 **동일한 언어는 번역 대상에서 제외**합니다. 모든 대상이 원문과 같으면 번역 생략 안내를 표시합니다. 혼합 문장 등 언어를 확정하기 어려우면 제외하지 않고 전체 대상으로 번역하며 이를 표시합니다. 모두 제외되는 경우에도 언어 감지를 위한 요청 1건은 실행됩니다. 요청 중 추가 단축키 입력은 중복 요청을 만들지 않습니다.

기존 기본 언어 이름 설정(한국어, English, 日本語, Español, 简体中文)은 `ko, en, ja, es, zh`로 읽어 들이며 사용자가 저장하기 전에는 설정 파일을 덮어쓰지 않습니다. 그 밖의 기존 언어 이름은 코드로 다시 지정해야 합니다.

일본어(`ja`) 번역에는 한자 위에 히라가나 **후리가나**를 표시합니다. 읽기는 같은 Gemini 요청으로 생성하며, 번역과 읽기 구문의 일치 여부를 검증한 뒤 안전한 ruby 요소로 렌더링합니다. AI가 생성한 읽기이므로 고유명사 등의 발음은 틀릴 수 있습니다.

번역창은 원래 앱의 포커스를 가져오지 않습니다. **Esc** 또는 ×로 닫으면 진행 중인 CLI 요청도 취소합니다. 닫기 전까지 결과를 확인하고 스크롤할 수 있습니다. 앱 종료는 트레이 메뉴의 **종료**를 사용하세요.

### macOS 권한

시스템 설정 → 개인정보 보호 및 보안 → **손쉬운 사용**에서 LBRX Translator를 허용해주세요. macOS가 **입력 모니터링**도 요청하면 함께 허용합니다. 권한 변경 후에는 앱을 완전히 종료하고 다시 실행해야 합니다. 개발 중 `npm run dev`를 사용하면 실행한 Terminal 등의 권한도 필요할 수 있습니다.

Finder에서 실행한 앱은 터미널의 셸 설정을 상속하지 않습니다. 기본 PATH 외에 Windows의 `%LOCALAPPDATA%\agy\bin`, macOS의 `~/.local/bin`과 일반 Homebrew 경로를 확인합니다. CLI를 찾지 못하면 설정에서 네이티브 `agy.exe` 또는 `agy` 실행 파일의 절대 경로를 지정하세요. 셸 스크립트나 명령행 인수는 경로 필드에 입력하지 않습니다.

### Gemini 구독과 API

Google은 [2026년 6월 18일부터 개인 Pro/Ultra의 Gemini CLI 지원을 종료](https://developers.googleblog.com/en/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)했습니다. 이 앱은 후속 공식 Antigravity CLI의 [headless 실행](https://antigravity.google/docs/cli/headless)을 사용합니다.

Google 로그인으로 캐시된 자격 증명이 있어야 합니다. 구독의 **Antigravity 한도와 제공 모델**이 적용되며 Gemini 웹 앱의 한도와 같다고 가정하지 않습니다. 별도 API 키나 추가 AI 크레딧으로 자동 전환하지 않습니다. 기본 모델은 `gemini-3.8-flash-low`이며, `agy models`에 표시되는 `gemini-` 모델 ID로 변경할 수 있습니다. 지원하지 않는 모델은 다른 모델로 조용히 대체하지 않고 오류를 표시합니다.

설정의 **CLI 설치 확인**은 버전 명령만 실행합니다. 로그인, 구독 상태, 네트워크 연결까지 검증했다는 의미는 아닙니다. 로그인 만료, 사용량 제한, 지원하지 않는 모델, 잘못된 응답, 시간 초과는 번역창에 오류로 표시합니다.

## 데이터 처리

- 전송에 동의한 뒤 단축키를 누른 경우에만 선택 문장과 대상 언어를 Google에 보냅니다. 민감한 내용은 선택해 번역하지 마세요.
- 번역 요청마다 임시 작업 폴더를 사용합니다. 텍스트는 셸 명령이나 프로세스 인수가 아닌 표준 입력으로 전달합니다.
- CLI의 파일 읽기·쓰기, 명령 실행, 웹 접근, MCP 도구 호출을 세션 전용 권한 규칙으로 거부합니다. 슬래시 명령과 스킬 확장도 막습니다. 기존 CLI 사용자 설정은 수정하지 않습니다.
- **Antigravity의 전역 규칙·훅·플러그인은 별도로 적용될 수 있습니다.** CLI에는 이를 모두 비활성화하는 문서화된 실행 옵션이 없으므로 앱이 완전한 격리를 보장하지 않습니다. 번역용으로 정리된 CLI 환경을 사용하고 전역 사용자 설정을 검토해주세요. 앱은 그 파일들을 임의로 수정하지 않습니다.
- 앱 자체는 원문과 번역 기록을 디스크에 저장하지 않습니다. **Antigravity의 로컬·서버 측 세션 기록과 Google의 데이터 처리·보관 정책**은 별도로 적용됩니다. CLI의 데이터 수집 설정도 계정에서 직접 관리해야 합니다.
- 대상 언어, 모델, 시간 제한, CLI 경로, 전송 동의 여부만 OS별 Tauri 설정 디렉터리의 `config.json`에 저장합니다. 잘못된 설정 파일은 자동 덮어쓰지 않고 오류를 표시합니다. 사용자가 설정을 저장해야 복구됩니다.

## 빌드

```powershell
npm run build
```

각 운영체제에서 네이티브로 빌드합니다. Windows에서는 `src-tauri\target\release\bundle`에 설치 파일이 생성됩니다. macOS에서는 `src-tauri/target/release/bundle`에 앱과 DMG가 생성됩니다. 배포용 서명과 macOS notarization은 별도 인증서 설정이 필요합니다. Antigravity CLI는 앱에 포함되지 않으므로 실행할 PC에도 설치해야 합니다.

설치 파일 없이 Windows 실행 파일만 만들려면:

```powershell
npm run build -- --no-bundle
```

### 자동 빌드와 배포

PR 및 `main` 푸시는 Windows/macOS 검증과 패키징을 실행합니다. 해당 Actions 실행의 **Artifacts**에서 결과를 30일 동안 내려받을 수 있습니다. 일반 푸시는 GitHub Release를 생성하지 않습니다.

`package.json`, npm/Cargo lockfile, `Cargo.toml`, `tauri.conf.json`의 버전을 동일하게 변경한 뒤 `v1.0.1`과 같은 버전 태그를 푸시하면 **Release** 워크플로가 두 운영체제의 릴리스 패키지를 빌드하고 GitHub Release에 자동 게시합니다. 어느 플랫폼이든 실패하면 게시하지 않으며, 태그와 매니페스트 버전이 다르면 빌드 전에 실패합니다.

```powershell
npm run check:version -- v1.0.1
git tag -a v1.0.1 -m "LBRX Translator v1.0.1"
git push origin v1.0.1
```

## 개발 확인

```powershell
npm run check
npm test
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo test --manifest-path src-tauri\Cargo.toml --lib
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
```

Rust 테스트는 단축키 상태 전이, 설정 저장과 검증, 다중 언어 응답 검증, 셸을 거치지 않는 CLI 호출, 시간 초과와 실패 응답을 확인합니다. CLI 호출 테스트는 로컬 Node.js fixture를 사용하므로 Google에 요청하거나 구독 한도를 사용하지 않습니다. 실제 OS의 전역 단축키, 권한, Google 로그인 동작은 해당 환경에서 별도로 확인해야 합니다.

실제 구독 계정 연결을 확인하는 테스트는 기본적으로 제외되어 있습니다. 아래 명령을 실행하면 영어·한국어·일본어·스페인어의 합성 문장으로 요청 4건을 보내므로 구독 한도가 사용됩니다. 각 원문과 동일한 언어가 제외되는지 확인합니다.

```powershell
cargo test --manifest-path src-tauri\Cargo.toml live_subscription_translation -- --ignored
```
