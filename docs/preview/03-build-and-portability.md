# 03 · 빌드 · 이식성

> 설계 명세 §3·§5.2 기반.

---

## 빌드 철학

**원문 소스 → 각 OS에서 빌드.** 사전 빌드 바이너리 배포에 의존하지 않는다.

이 원칙이 의미하는 세 가지:

1. **순수 Rust 코어 + 최소/제로 C 의존성.** 외부 C 라이브러리 의존을 피해 3-OS 빌드 마찰을 제거한다.
2. **`cargo`가 유일한 빌드 진입점.** `cmake`, `make`, 별도 빌드 스크립트 없이 `cargo build`/`cargo test` 하나로 전체 워크스페이스가 빌드·테스트된다. (구조적 schema 해시는 proc-macro로 산출하며 build.rs 셸아웃이 아니다 — 이 원칙을 지킨다. 아래 "구조적 schema 해시 빌드" 참조.)
3. **MSRV 고정.** `rust-toolchain.toml`로 Rust 버전을 저장소에 고정해 재현성을 확보한다.

**"소스에서 빌드"는 *빌드 출처* 규칙이지 *실행 모델*이 아니다.** 이 철학은 (a) 벤더가 사전
빌드한 불투명 바이너리 blob에 의존하지 않고 모든 설치가 타깃 OS에서 소스를 컴파일한다는
뜻이지, (b) 사용자가 소스 체크아웃에서 `cargo run`으로 돌린다는 뜻이 아니다. 흐름은
**소스 빌드로 바이너리를 *산출***(`cargo build --release`/`cargo install`)한 뒤, 그
바이너리를 `~/.cargo/bin`에 설치해 **터미널 명령**(`vita`/`vcmp`/`velab`/`vrun`)으로
실행하는 것이다 — ripgrep·fd·uutils-coreutils와 같은 모델. `cargo run`은 컨트리뷰터 개발
워크플로이며 최종 사용자 경로가 아니다.

---

## Cargo Workspace 구조

spec §5.2의 코어 11개 크레이트 + 단계별 산출물 계층 3개(`vita-artifact`, `vita-artifact-derive`, `vita-schema`) + 운영 로깅 1개(`vita-log`) = 15개 크레이트를 단일 cargo workspace에 배치한다. 각 크레이트는 단일 책임 + 명확한 인터페이스로 분리해 독립 테스트가 가능하다.

```toml
# Cargo.toml (워크스페이스 루트)
[workspace]
members = [
    "crates/hdl-preprocess",  # 컴파일러 지시어 처리, 매크로 전개, include
    "crates/hdl-lexer",       # 토큰화 (언어별 토큰 집합)
    "crates/hdl-parser",      # 토큰 → AST (언어별)
    "crates/hdl-ast",         # 언어별 AST 타입 정의
    "crates/elaborate",       # 파라미터 해소·계층 평탄화·타입/연결성 검사 → IR
    "crates/sim-ir",          # 언어 중립 시뮬레이션 IR
    "crates/sim-engine",      # 이벤트 구동 커널, 스케줄러, 시간 모델
    "crates/hdl-builtins",    # 표준 $-system tasks/functions 라이브러리
    "crates/vcd-writer",      # IEEE 1364 VCD 직렬화
    "crates/diag",            # 진단/오류 리포팅 (소스 위치, 메시지)
    "crates/vita-artifact",        # 단계 산출물 (역)직렬화·헤더·버전·staleness·--dump
    "crates/vita-artifact-derive", # #[derive(SchemaHash)] proc-macro (구조적 형상 해시)
    "crates/vita-schema",          # SchemaShape trait + ShapeRegistry + blake3 (형상 해시 런타임; leaf) — 16
    "crates/vita-log",             # 운영 로깅/transcript/severity 라우팅/로그 sink/exit-code
    "crates/cli",             # 드라이버 바이너리: vita(원샷) + vcmp/velab/vrun(단계별)
    # ── dev/test 전용 (publish=false, 배포 multicall·설치 대상 아님 — 위 15개 프로덕션 그래프와 별개) ──
    "crates/vcd-diff",        # 정규화 VCD diff (차등검증 — 09)
    "crates/corpus-runner",   # 컴플라이언스 코퍼스 러너 (09)
]
resolver = "2"

[workspace.package]
edition = "2021"          # edition 2024는 rustc>=1.85 필요 — MSRV 1.82와 비호환이라 2021 고정 (16/PR1-B)
rust-version = "1.82"     # MSRV — 채택 miette 7.6.0 manifest(1.82.0)가 상한
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/vitamin"  # placeholder

[workspace.dependencies]
# 공통 의존성을 여기서 버전 고정 (각 크레이트는 workspace = true 로 참조)
logos  = { version = "0.16", default-features = false }   # semver range: >=0.16.0, <0.17.0 (패치 업데이트 허용)
winnow = "1"                                              # 파서 부트스트랩(1.0.3+); hot·복구-임계 규칙은 hand-RD로 이관. 복구는 unstable-recover 피처
# (chumsky/ariadne 제거 — chumsky GitHub archived 2026-04-02, ariadne 미채택. 근거 02 참조)
# ── 단계별 산출물 / CLI (모두 순수 Rust — C 의존 없음) ────────────────────
serde   = { version = "1", features = ["derive"] }       # 직렬화 경계 trait (additive derive)
postcard = { version = "1", features = ["use-std"] }     # 단일 바이너리 인코더 (bincode 폴백 없음)
blake3  = "=1.8.2"                                       # 형상·소스·매니페스트 다이제스트 (단일 계열); 1.8.3+는 edition 2024(rustc>=1.85)라 핀 (PR1-B)
toml    = "0.8"                                          # work/lib.toml 매니페스트
ron     = "0.8"                                          # --dump full-precision 텍스트 뷰
clap    = { version = "4", features = ["derive"] }       # CLI 파서 (multicall은 손수 argv[0] 디스패치)
# vita-artifact-derive 전용 (proc-macro 크레이트)
syn         = { version = "2", features = ["full", "extra-traits"] }
quote       = "1"
proc-macro2 = "1"
# ── 운영 로깅 (vita-log; 모두 순수 Rust, C 의존 없음) ──────────────────────
tracing            = "0.1"                              # 구조화 이벤트 스트림
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }  # terminal/file 레이어
miette             = { version = "7", default-features = false }  # 코드/url 진단 (leaf diag는 IO-free; fancy는 vita-log에서만 활성화)
# CI 골든-포맷 가드 전용 (런타임 의존 아님 — dev-dependencies)
# serde-reflection = "0.6"   # cargo test에서 wire 포맷 표류 검출 (0.4는 stale; 0.6 = edition 2021, MSRV 1.72)
```

> **miette 피처 계층화.** 워크스페이스 기본은 `default-features = false`로 두어 leaf `diag`를 IO-free로 유지한다(`diag/Cargo.toml`: `miette = { workspace = true }` — 순수 데이터 모델만). 터미널 렌더링이 필요한 `vita-log`만 자기 `Cargo.toml`에서 `miette = { workspace = true, features = ["fancy"] }`로 `fancy`를 가산한다(Cargo 피처는 additive라 한 크레이트의 가산이 다른 크레이트의 leaf 순수성을 깨지 않는다).

### 크레이트 의존 방향

```
cli                  ──► 전부 + vita-artifact + vita-log
vita-log             ──► diag, vita-artifact, sim-ir, tracing, tracing-subscriber, miette
vita-artifact        ──► hdl-ast, sim-ir, hdl-preprocess, diag, vita-artifact-derive, vita-schema
hdl-builtins         ──► sim-ir, sim-engine, vcd-writer, diag(&dyn LogSink)
sim-engine           ──► sim-ir, diag(&dyn LogSink)
elaborate            ──► hdl-ast, sim-ir, diag(&dyn LogSink)
hdl-parser           ──► hdl-lexer, hdl-ast
hdl-lexer            ──► hdl-preprocess
hdl-ast / sim-ir     ──► vita-artifact-derive, vita-schema   (serde·SchemaHash derive + 런타임 trait)
vita-artifact-derive ──► (leaf: syn / quote / proc-macro2)
vita-schema          ──► (leaf: blake3)          # SchemaShape trait·ShapeRegistry — 순환 회피용 별도 leaf (16)
```

`diag`는 `LogSink` trait + `Severity`/`MsgCode`/`Frame`/`Diagnostic`/`LogEvent` 데이터 모델을
보유하되 IO·tracing 의존이 없어 **leaf로 남는다**. emitter 크레이트(elaborate·hdl-builtins·
sim-engine·vita-artifact)는 `&dyn LogSink`만 받아 **`diag`에만 의존**하고, 구체 tracing sink를
만들어 설치하는 크레이트는 `cli`뿐이다 — 그래서 `vita-log → vita-artifact`가 비순환을 유지한다.
상세 [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md).

`diag`·`hdl-preprocess`·`vita-artifact-derive`·`vita-schema`가 최하위 leaf 크레이트다 — 다른 워크스페이스 크레이트에 의존하지 않아 독립 테스트가 가장 쉽다. `hdl-ast`·`sim-ir`는 D2 도입으로 `serde`·`SchemaHash` derive를 위해 `vita-artifact-derive`(proc-macro)와 `vita-schema`(런타임 `SchemaShape` trait + `ShapeRegistry`)에 의존하게 되어 더 이상 순수 leaf는 아니지만, 여전히 다른 HDL/백엔드 크레이트와 무관해 격리 테스트가 쉽다. `vita-artifact-derive`는 `[lib] proc-macro = true`이며 어떤 워크스페이스 크레이트에도 의존하지 않는다(빌드그래프 leaf). `vita-schema`는 `blake3`에만 의존하는 별도 런타임 leaf로, **trait를 `vita-artifact`에 두면 `sim-ir → vita-artifact → sim-ir` 순환이 되므로** 분리했다(상세 [16-schema-hash-spec.md](16-schema-hash-spec.md)).

---

## 바이너리 산출 — multicall + `[[bin]]`

네 명령(`vita`/`vcmp`/`velab`/`vrun`)은 무거운 파이프라인 전체(preprocess..sim-engine..
vcd-writer..hdl-builtins..vita-artifact)를 정적 링크하므로, 4개 별도 바이너리는 빌드·링크·
strip·서명 비용이 ~4배다. 따라서 **프로덕션은 단일 multicall 바이너리**로 배포하고,
단계별 디버깅용 4개 `[[bin]]`은 dev 전용 피처 뒤에 둔다.

- **디스패치는 손수 구현한다.** clap `Command::multicall(true)`는 argv[0]을 떼고 다음 토큰을
  *서브커맨드 이름*으로 파싱하므로 `vita top.sv`(positional 소스 파일을 받는 기본 applet)와
  양립하지 않는다. `main()`이 `std::env::args_os().next()`의 베이스네임을 읽어
  `{vcmp,velab,vrun}`이면 해당 applet의 clap Command로, 아니면 positional `<source-files>...`를
  받는 `vita` 원샷 파서로 분기한다(`vita vcmp …` 명시형도 인식).
- 설치 시 실제 바이너리는 `vita` 하나뿐이고 `vcmp`/`velab`/`vrun`은 하드링크(폴백:
  심볼릭 링크) → `vita`다. argv[0] 변조(복사·개명) 시 명시형 `vita <applet> …`로 폴백.

```toml
# crates/cli/Cargo.toml
[[bin]]
name = "vita"
path = "src/main.rs"          # multicall main: argv[0] 베이스네임 디스패치 (손수)

[features]
separate-bins = []            # dev 전용 — 기본 빌드는 vita 단일 바이너리만 산출

[[bin]]
name = "vcmp"
path = "src/bin/vcmp.rs"
required-features = ["separate-bins"]
# velab / vrun 동일 패턴 (required-features = ["separate-bins"])
```

각 `src/bin/<stage>.rs`는 `cli::run_<stage>()`를 호출하는 3줄 shim이며, 실제 로직은 cli
라이브러리에 있어 multicall `main()`과 dev shim이 동일 코드 경로를 공유한다(분기 없음).

---

## 릴리스 프로파일

```toml
# Cargo.toml (워크스페이스 루트)
[profile.release]
opt-level     = 3         # IR-walking 이벤트 루프가 throughput-critical — size 's'/'z' 금지
lto           = "thin"    # 15-크레이트 파이프라인 교차 인라인; fat-LTO 속도를 낮은 링크 비용으로 회수
codegen-units = 1         # hot 인터프리터 루프 최대 최적화 (LTO와 짝)
strip         = "symbols" # 작은 배포 바이너리 + 깔끔한 macOS 공증; multicall이라 strip 대상 1개
# panic = "abort" 를 설정하지 않는다 — 기본 unwind 유지.

[profile.dev.package."sim-engine"]
opt-level = 2             # 디버그 빌드에서도 엔진 내부 루프만 최적화 (나머지는 debuginfo 유지)
```

**`panic = unwind` 유지 근거(앱은 보통 abort라는 통념과 의도적 이탈, 문서화 필수):** 엔진
불변식이 런 도중 깨질 때 **부분 VCD를 flush하고** diag 형식 내부오류 리포트를 내려면 cli
경계에서 `catch_unwind`가 필요한데, `panic=abort`는 이를 금지한다. 더 작은 바이너리/빠른
시작이라는 abort의 이점은 차등검증 도구의 "crash 시 부분 파형 보존"(05 상시 횡단 VCD golden
diff)을 잃는 대가를 넘지 못한다. 향후 JIT/no_std hot path가 abort를 요구하면 그때
프로파일별로 분리한다. `strip` 후 provenance는 split-debuginfo(별도 dSYM/.debug)로 보존한다.
fat-LTO는 필요 시 태그 릴리스에서 `cargo build --profile dist`로만 선택한다 — MVP는 thin으로
충분하므로 별도 `[profile.dist]`를 두지 않는다.

---

## 설치 · 호출

```bash
# 빌드 (소스에서 — 철학): --locked로 커밋된 Cargo.lock 사용 → 3-OS 재현 빌드
cargo build --release --workspace --locked   # → target/release/vita (단일 multicall)

# 설치 (사용자 1차 계약): vita를 ~/.cargo/bin 에 — rustup이 PATH에 이미 추가
cargo install --path crates/cli --locked     # 기본 빌드는 [[bin]] 하나(vita)뿐이라 비모호
# git/멀티패키지 소스에서는 패키지 셀렉터(-p)로 (always 비모호):
cargo install --git https://github.com/<org>/vitamin -p cli --locked

# 단계 명령 링크 팜 (설치 스크립트가 자동 생성): vcmp/velab/vrun → vita
#   하드링크 우선(서명 공유), 실패 시 심볼릭 링크
for s in vcmp velab vrun; do ln -f "$(command -v vita)" "$(dirname "$(command -v vita)")/$s"; done

# 호출 — 항상 설치된 바이너리를 터미널 명령으로 (cargo run 아님)
vita  top.sv                                  # 원샷 (메모리 스트리밍, 디스크 산출물 없음)
vcmp  -y ./rtl --work work=./work *.sv        # 단계: compile
velab -s top -o top.velab                     #       elaborate
vrun  top.velab +SEED=1                        #       simulation (상류 체인 라이브 재검증)
```

`-p cli` 패키지 셀렉터를 쓴다(오직 cli만 *배포/설치* 바이너리를 가지므로 항상 비모호; `vcd-diff`/`corpus-runner`는 dev/test 전용으로 `publish = false`라 설치·multicall 대상이 아니다). `--bin vita`는
`separate-bins` 피처로 설치할 때만 추가로 필요하다. rustup 설치 시 `~/.cargo/bin`이 PATH에
들어간다(다른 방식 설치 시 `export PATH="$HOME/.cargo/bin:$PATH"`). MVP는 system-wide
`/usr/local/bin` 설치가 불필요하다.

---

## 배포 채널

| 채널 | 명령/위치 | 비고 |
|---|---|---|
| **1차 — cargo install (소스 빌드)** | `cargo install --git … -p cli --locked` | 철학 그대로, MVP가 요구하는 유일 채널. macOS는 quarantine 속성이 안 붙어 미서명 바이너리도 Gatekeeper 프롬프트 없이 실행 |
| 2차 (게시 후) | `cargo install vitamin-cli` (crates.io) | 동일 소스-빌드 의미. cargo-binstall은 소스 빌드로 폴백 |
| 선택 편의 | `dist`(cargo-dist) GitHub Releases 타르볼 | per-OS **소스 빌드**를 CI에서 캐시한 것(벤더 prebuilt 아님). `cargo-zigbuild`로 old-glibc(RHEL8 2.28/RHEL9 2.34) 대응, 타르볼에 multicall 링크 팜 포함. **단 다운로드 타르볼은 quarantine** → 미서명 mach-o는 Gatekeeper 차단이므로 codesign(Developer ID)+공증 또는 `.pkg`/`.dmg` 필요(바이너리 단독 타르볼은 공증 불가). 05 배포-하드닝 단계로 연기 |
| 연기 (post-MVP) | deb(`cargo-deb`)·rpm(`cargo-generate-rpm`)·Homebrew tap | system-wide 설치 + OS 패키지 매니저 갱신. RHEL 타깃 대상층에 rpm이 의미 |

소스-빌드가 1차로 남는 한 dist 타르볼은 벤더 prebuilt 의존이 아니라 *편의 캐시*이므로 빌드
철학 안에 머문다.

---

## 구조적 schema 해시 빌드 (D2)

직렬화 타입 형상 해시를 cargo-native로 산출하는 3계층(상세
[14-staged-artifacts.md](14-staged-artifacts.md) §5):

- **Layer 1 — proc-macro `vita-artifact-derive`**: `#[derive(SchemaHash)]`가 syn AST(필드·
  variant + **serde 속성**)를 walk해 `blake3` 형상 해시 const를 emit. proc-macro는 rustc
  안에서 도므로 build.rs 셸아웃·codegen 없이 03 원칙을 지킨다. **결정론 필수** —
  Vec/BTreeMap·정렬된 레지스트리만 쓰고 HashMap을 금지해 OS/arch 무관 바이트 동일을 보장한다
  (`--locked` 재현성). serde 속성(`rename`/`skip`/`with`/…)도 해시에 넣어 wire 편차를
  런타임에 포착한다.
- **Layer 2 — 빌드 지문**: `option_env!("VITA_GIT_SHA")`/`option_env!("VITA_GIT_DIRTY")`/
  `env!("CARGO_PKG_VERSION")`/`cfg!(debug_assertions)`로 git sha+dirty+profile을 헤더에 stamp.
  CI/설치 래퍼가 env 주입, 평범한 `cargo build`엔 없으면 graceful "unknown". **빌드 스크립트
  0개**, provenance 전용(staleness 키 아님 — dirty 트리가 재컴파일을 강제하지 않는다). sha가
  꼭 필요해지면 셸아웃 없는 `vergen-gix` build.rs(순수 Rust `gix`)가 유일 허용 예외.
- **Layer 3 — CI 골든 가드**: `serde-reflection`을 dev-dependency로 두고 `cargo test`에서
  골든 Registry(RON)와 diff해 wire 포맷 표류를 잡는다(런타임 codegen 아님).

---

## MSRV · Toolchain 고정

### `rust-toolchain.toml`

저장소 루트에 배치. `cargo build`·`rustup run` 실행 시 자동으로 이 버전을 사용한다.

```toml
# rust-toolchain.toml (저장소 루트)
[toolchain]
channel = "1.82.0"          # MSRV (miette 7.6.0 → 1.82); stable 릴리스 고정
components = [
    "rustfmt",              # 코드 포맷
    "clippy",               # 린트
    "rust-src",             # IDE 지원용 (rust-analyzer)
]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]
```

### MSRV 정책

- **현재 MSRV: 1.82.** 채택한 miette 7.6.0의 manifest MSRV가 `1.82.0`이라 이 값으로 고정한다. Rust 1.80이 안정화한 `std::sync::LazyLock`을 포함한다(참고: `let-else`는 1.65, `OnceLock`은 1.70).
- **채택 크레이트 MSRV 집합의 상한으로 결정.** 현재 상한 = miette(1.82). winnow 1.x·logos 0.16(1.80)·codespan 0.13(1.67)은 모두 1.82 이하. 렌더 백엔드를 codespan으로 스왑하면 1.67까지 내려갈 수 있다.
- **MSRV 변경은 semver minor bump로 처리.** 패치 릴리스에서 MSRV를 올리지 않는다.
- **CI에서 MSRV 최소 버전으로 `cargo check` 실행** — `rust-toolchain.toml` 외에 별도 `toolchain: "1.82.0"` 잡을 매트릭스에 포함해 MSRV 회귀를 방지한다.

---

## 3-OS 매트릭스

| OS | 패키지 매니저 | Rust 설치 방법 | 비고 |
|---|---|---|---|
| Ubuntu LTS (22.04/24.04) | `apt` | `rustup` | 기준 플랫폼. CI `ubuntu-latest` 러너에서 상시 검증. glibc 2.35+ 대응. |
| RHEL 8/9 계열 | `dnf` | `rustup` | glibc 2.28+ (RHEL8) / 2.34+ (RHEL9) 호환. CI는 UBI(Universal Base Image) 컨테이너 사용 권장 (self-hosted 러너 없이도 재현 가능). RHEL8에서 `gcc` 링커 필요 시 `dnf install gcc` 사전 설치. |
| macOS Apple Silicon + Intel | `brew` (선택) | `rustup` | `aarch64-apple-darwin` + `x86_64-apple-darwin` 양 target 빌드 검증. universal binary(lipo)는 별도 후속 작업. CI는 `macos-latest`(Apple Silicon) 러너 사용. |

**공통 설치 흐름:**

```bash
# 1. rustup 설치 (모든 OS 동일)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 2. 저장소 클론 후 빌드 (rust-toolchain.toml이 버전 자동 선택)
git clone https://github.com/your-org/vitamin
cd vitamin
cargo build --workspace
cargo test --workspace
```

---

## CI 매트릭스

GitHub Actions 기준. Ubuntu/macOS 네이티브 잡과 RHEL UBI 컨테이너 잡을 분리한다. 두 잡(`build-native`, `build-rhel`)은 병렬 실행된다 — `container:` 필드에 빈 문자열을 넣으면 Actions가 이미지 이름으로 해석해 오류가 발생하므로, 컨테이너가 필요 없는 러너와 필요한 러너는 별도 잡으로 구분한다.

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  # ── Ubuntu + macOS (네이티브 러너, 컨테이너 없음) ────────────────────────
  build-native:
    name: Build & Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}

    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@v1
        with:
          # rust-toolchain.toml의 channel을 그대로 읽음
          toolchain: "1.82.0"
          components: rustfmt, clippy

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build (all crates)
        run: cargo build --workspace --locked

      - name: Test (all crates)
        run: cargo test --workspace --locked

      - name: Clippy
        run: cargo clippy --workspace -- -D warnings

      - name: Fmt check
        run: cargo fmt --all -- --check

  # ── RHEL9 UBI 컨테이너 (GitHub-hosted ubuntu 러너에서 실행) ──────────────
  build-rhel:
    name: Build & Test (RHEL9/UBI)
    runs-on: ubuntu-latest
    container: redhat/ubi9

    steps:
      - uses: actions/checkout@v4

      - name: Install C linker
        run: dnf install -y gcc

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: "1.82.0"
          components: rustfmt, clippy

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target/
          key: rhel9-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build (all crates)
        run: cargo build --workspace --locked

      - name: Test (all crates)
        run: cargo test --workspace --locked

  # ── MSRV 회귀 방지 잡 (Ubuntu만으로 충분) ───────────────────────────────
  msrv:
    name: MSRV check (Rust 1.82)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: "1.82.0"
      - run: cargo check --workspace --locked
      # (cargo audit 단계는 별도 job으로 추후 추가)
```

**RHEL CI 전략 선택 근거:** `redhat/ubi9` 공개 이미지를 컨테이너로 실행하면 GitHub-hosted 러너(`ubuntu-latest`)에서도 RHEL9 glibc·패키지 환경을 재현할 수 있다. self-hosted 러너 구성 없이 CI를 유지할 수 있어 초기 단계에 실용적이다. 프로젝트가 성숙하면 RHEL8 호환을 위해 `redhat/ubi8`을 추가하거나 self-hosted 러너로 전환한다.

---

## 외부 의존성 정책

1. **순수 Rust crate 우선.** 동일 기능이 순수 Rust 크레이트로 충족된다면 C 바인딩 크레이트를 채택하지 않는다.
2. **C 라이브러리 의존은 최대한 회피.** 불가피할 경우:
   - `build.rs`에 명시적으로 기록.
   - 3-OS 빌드 검증을 CI에 추가.
   - 해당 의존성의 정적 링크 가능 여부 확인 (`*-sys` 크레이트 사용 시 vendored feature 선호).
3. **`Cargo.lock` 커밋.** 바이너리 크레이트(`cli`)의 `Cargo.lock`을 저장소에 포함해 CI에서 `--locked` 빌드를 사용한다. 라이브러리 크레이트의 lock 파일은 관례대로 `.gitignore`에 포함하지 않는다.
4. **의존성 감사.** `cargo audit`를 CI에 추가해 알려진 취약점을 주기적으로 점검한다.

---

## Sources

- 본 spec §3 — `/docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`
- 본 spec §5.2 — 위 동일 파일 (Cargo 워크스페이스·크레이트 표)
- `02-implementation-language.md` — MSRV 결정 맥락
- GitHub Actions 문서: https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners
- Red Hat UBI 이미지: https://catalog.redhat.com/software/containers/ubi9/ubi/615bcf606feffc5384e84400
- dtolnay/rust-toolchain action: https://github.com/dtolnay/rust-toolchain
