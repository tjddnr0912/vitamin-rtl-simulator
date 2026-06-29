# 13 · 진단 · 로깅 · 트랜스크립트

> 운영 로깅의 권위 문서다. 상용 도구(Cadence/Synopsys)처럼 모든 동작 정보(읽는 파일,
> 라이브러리 해소, elaborate 진행, 런 요약)를 터미널에 출력하고 로그 파일에 남기며,
> error/warning/fatal을 소스 위치(file:line:col + include/instance 경로)까지 추적해
> 디버깅할 수 있게 한다. 진단 *렌더링*은 `diag` 크레이트, 운영 *transcript·로그파일·
> severity·exit-code*는 `vita-log` 크레이트가 담당한다. 04는 요약, 본 문서가 권위다.

---

## 두 계층 — `diag`(렌더러) vs `vita-log`(운영 계층)

깨끗한 seam으로 책임을 가른다:

- **`diag` = 렌더러.** 진단 하나 `{severity, code, message, location?, context: Frame[], sim_time?}`를
  받아 file:line:col + caret 밑줄 스니펫(miette `fancy`/codespan 스타일)으로 그린다. **하나의
  진단을 그릴 뿐** transcript·로그파일·카운트·exit-code를 모른다. severity/메시지 코드/Frame
  데이터 모델과 **`LogSink` trait**을 보유하지만 **IO·tracing 의존이 없어 leaf로 남는다.**
- **`vita-log` = sink + 정책.** 모든 출력 주체(vcmp/velab/vrun 진행, parse 에러, elaborate
  다중구동, RTL `$error`, `$display`, staleness 거부)가 `LogEvent` 하나를 방출하고 **누구도
  `println!`을 직접 호출하지 않는다.** 이 계층이 severity 라우팅, 안정 메시지 코드 레지스트리,
  카운트, suppress/promote 게이트, 터미널+로그파일 tee, exit-code 계산, 배너/진행/filelist
  echo, §7 side-table→span 브리지를 소유한다.

emitter(`elaborate`/`hdl-builtins`/`sim-engine`/`vita-artifact`)는 `&dyn LogSink`만 받으므로
**`diag`에만 의존**(이미 의존)하고 tracing 머신을 끌어오지 않는다. 구체 sink를 만들어 설치하는
유일한 크레이트는 `cli`다. (LogSink trait이 `diag`에 있어야 emitter가 vita-log를 의존하지 않고
dep 그래프가 비순환으로 유지된다.)

---

## 단일 이벤트 모델 (단일 진실 공급원)

```rust
enum LogEvent {
  Diagnostic(Diagnostic),  // -> diag가 본문(span/caret/related) 렌더
  Progress(ProgressEvent), // 배너, "reading file X", "elaborating top", 런 요약
  RtlOutput(RtlText),      // $display/$write/$monitor/$strobe — 사용자 텍스트, severity 없음
}
struct Diagnostic {
  severity: Severity,          // Note | Info | Warning | Error | Fatal
  code: MsgCode,               // 안정 enum, 예: E_ELAB_MULTIDRIVER
  message: String,
  location: Option<SourceLoc>, // compile은 span 직접; runtime은 §7 side-table로 복원
  context: Vec<Frame>,         // include/macro/-f 확장 스택, instance/hierarchy 경로
  sim_time: Option<TimeStamp>, // runtime severity 이벤트에만 (IEEE 요구)
}
```

**sink 아키텍처:** 하나의 이벤트 스트림을 `tracing` + `tracing-subscriber` 위의 두 포매터로
fan-out — **터미널 레이어**(TTY/색상, `--color`/`NO_COLOR` 인지)와 **파일 레이어**(ANSI 무조건
제거). 둘 다 *같은* 스트림을 소비하므로 터미널과 로그파일이 **절대 어긋날 수 없다**(vvp의
stdout/stdlog 이중 채널 drift 교훈).

> **단일 writer tee (확정 설계).** `$display`/`$write`/`$monitor`(throughput hot path)는
> tracing의 이벤트별 dispatch 오버헤드를 피해 **직접 buffered writer**로 가고, `Diagnostic`/
> `Progress`만 tracing을 거친다. 단 — **그 buffered writer는 터미널과 `--log` 둘을 먹이는
> *단일* writer**여야 하며(독립 writer 2개 절대 금지), tracing이 만든 Diagnostic도 *같은*
> 정렬 writer로 방출 시점에 flush해 sim-time 인터리브를 보존한다. 즉 빠른 경로와 진단 경로가
> 출력 *생성*은 다르되 출력 *지점*은 하나다. 이로써 성능(hot-path tracing 우회)과 단일-진실
> (drift 불가)을 동시에 얻는다. MVP를 전부-tracing(A)로 시작해도 무방하나, 이 단일-writer
> 불변식만은 1일차부터 지켜 미래 최적화가 깰 수 없게 한다.

---

## Severity lattice (5단계)

| 레벨 | 출력 토큰 | 런 효과 |
|---|---|---|
| Fatal | `fatal[<CODE>]:` | 단계 **즉시 중단**, nonzero exit. compile-time=차단 parse/elab 또는 버전 게이트; runtime=`$fatal`; elaborate-time=`$fatal` |
| Error | `error[<CODE>]:` | **기록 + 계속**(에러 복구); 단계 실패 플래그; 단계 끝 nonzero; `--error-limit` 대상 |
| Warning | `warning[<CODE>]:` | 출력 + 계속; `-Werror[=CODE]` 승격 시에만 nonzero |
| Info | `info[<CODE>]:` | 정보(`$info`); 계속; exit 영향 없음; 억제 가능 |
| Note | (부모에 첨부 Frame) | 맥락; 독립 fatal 불가 |

- **continue-vs-abort는 IEEE를 따른다**(RTL 태스크): `$info`/`$warning`/`$error`는 계속,
  `$fatal`만 중단.
- `--error-limit N`(Verilator 기본 50; warning 미포함)은 N개 Error 후 단계를 중단해 깨진
  파일이 수천 줄을 뱉지 못하게 한다(한도 도달 자체가 Fatal `F-LIMIT-ERRORS`).
- **counts summary**: sink가 severity별 카운트를 누적하고 단계/런 epilogue가 항상
  `errors=E warnings=W notes=N`을 출력(억제 불가 spine). vrun은 추가로 종료 원인
  (`$finish`/`$fatal`/`--finish-at`/`$stop`)을 보고.
- 단계 드라이버(vcmp/velab)는 Error가 하나라도 있으면 **nonzero 반환**해 Makefile/CI가 다음
  단계 전에 멈춘다(xrun ELBERR / VCS "no simv" 선례).
- severity 정책 상태(카운트·한도·승격)는 전부 vita-log에 있고 **artifact에 절대 안 들어간다**
  — 순수 bucket C.

---

## 메시지 코드 (`VITA-####`)

모든 진단에 **안정·네임스페이스 mnemonic**을 부여한다 — Xcelium(`SVVMAP`/`ELBERR`)·
Verilator(`WIDTH`/`BLKANDNBLK`)에서 가장 베껴올 가치가 큰 기능이며, 자유 텍스트뿐인 현
`diag`에 vita-log가 더하는 net-new 능력이다.

- **스킴:** grep 친화 출력형 `VITA-E####`/`W####`/`I####`/`F####` + Rust 친화 dotted enum 이름.
- **카테고리(자기설명):** `E-PP-INCLUDE-NOT-FOUND`, `E-PARSE-UNEXPECTED-TOKEN`,
  `E-ELAB-MULTIDRIVER`/`E-ELAB-PORT-MISMATCH`/`E-ELAB-UNRESOLVED-INSTANCE`,
  `F-RUN-FATAL`/`E-RUN-USER-ERROR`/`W-RUN-USER-WARNING`/`I-RUN-USER-INFO`/`E-RUN-RANGE`/
  `E-RUN-ASSERT-FAIL`(action block 없는 assert 실패 — SVA Phase 2 예약),
  `F-ELAB-USER-FATAL`/`E-ELAB-USER-ERROR`/…(elaboration-시점 severity),
  `E-FLIST-CYCLE`/`E-FLIST-DEPTH`/`E-FLIST-GLOB`/`E-FLIST-UNDEF-ENV`/`E-FLIST-NOT-FOUND`/
  `E-FLIST-WRONG-STAGE`/`E-FLIST-DUP-CTX-CONFLICT`/`W-FLIST-MIXED-BASE`/`W-FLIST-OVERRIDE`
  (단일값 knob 충돌 override — always-logged)/`E-DUP-UNIT`,
  `E-ART-SCHEMA-MISMATCH`/`E-ART-STALE-UPSTREAM`/`E-ART-VERSION-GATE`/`E-ART-FORMAT-MISMATCH`.
- **구현:** 코드는 `diag` 안의 **exhaustive 컴파일러 체크 enum**(stringly-typed 아님 — drift
  없음). 각 variant가 출력 태그·기본 severity·짧은 제목을 보유, miette `code()`/`url()`이 위에
  올라탄다. `diag`에 있으므로 이미 diag를 의존하는 모든 크레이트가 새 의존 없이 타입 진단을
  만든다.
- **용도:** ① 코드별 suppress/promote(`-Wno-<CODE>`/`-Werror=<CODE>`) ② `vita explain <CODE>`
  (nchelp/`rustc --explain` 류) ③ corpus/JUnit이 깨지기 쉬운 메시지 텍스트가 아니라 **코드로
  assert**(09: substring이 아니라 `E-ELAB-MULTIDRIVER` 발화를 검증).

- **권위 카탈로그 + 거버넌스:** 각 코드의 **원인·예시·해결**은 별도 산출물
  [15-error-code-reference.md](15-error-code-reference.md)에 정의한다(`vita explain`의 소스).
  번호대는 카테고리별 예약(`0`=general, `1`=PP, `2`=PARSE, `3`=ELAB, `4`=RUN, `5–7`=예약,
  `8`=FLIST, `9`=ART), severity 접두 E/W/I/F. **mnemonic이 1차 안정 키**(영구 불변, renumber
  불가), 숫자는 보조이며 부여 후 영구. **CI 동기 게이트**: `MsgCode` enum과 15의 코드 집합이
  1:1이어야 빌드 통과 — *새 에러를 추가하면 15에 항목 추가 없이는 빌드가 깨진다*(문서 동기화 강제).
  게이트 대상 = `MsgCode` enum에 실재하는 코드 = 15 본문 §0–9 full-entry뿐이다. 15 부록 A의 예약
  코드는 enum 미등재이므로 게이트에서 제외(승격 시 enum 등재와 동시에 편입 — 15 거버넌스).

---

## 단계별 트랜스크립트

기본 정책 = **quiet-on-success**(iverilog/Verilator 습관). 클린 런이면 배너 + 단계당 1줄 요약,
상세는 `-v`/`-vv`. 네 드라이버 모두 인식 가능한 배너를 찍어 xrun/GHDL/iverilog 사용자가
익숙하게 한다.

- **vcmp** — 기본: 배너; 단위별 `compiling rtl/a.sv`; 논리 work-lib 해소(`work -> ./work`,
  D3 `library:unit`); 꼬리 요약 `compiled N units into work (M modules, K packages)` + 각 `.vu`
  기록. `-v`: **`-f` 전개 후 정렬된 filelist**(RULE S로 해시될 순서를 사용자가 보도록 항상
  -v에 출력), include 스택, 유효 `+define+` 집합, `` `timescale ``/`` `default_nettype `` 상속.
  `-vv`: 매크로 확장 트레이스.
- **velab** — 기본: 배너; `top resolved: <top>`; `hierarchy flattened: N instances`; 파라미터
  오버라이드; `global_time_precision = min(...)`; 다중구동 검출(수); `snapshot <top>.velab
  written`. `-v`: 인스턴스별 진행, 각 `-L` 바인딩 + 소비 트리플, defparam 해소.
- **vrun** — 기본 고정 순서: ① **RULE V 재검증 배너 먼저** — `re-checking upstream chain
  against live source…` → `upstream OK (N units, manifest match)` 또는 거부 진단; ② 본문 =
  `$display`/`$write`/`$monitor`/`$strobe` + severity 줄을 sim-time 순으로 인터리브; ③ 런 요약
  `simulation finished at <sim_time> via $finish | errors=E warnings=W notes=N`. stale 상류면
  시뮬 안 하고 `E-ART-STALE-UPSTREAM` + 힌트 `rerun vcmp/velab or vrun --rebuild`.
- **vita** — union 스트리밍, staleness/재검증 배너 없음(디스크 산출물이 없어 stale 대상 없음,
  §4 인메모리 동치). 단계 배너가 하나의 연속 transcript로.

**filelist echo(횡단):** `-f`/`-F` 전개 순서가 load-bearing(RULE S)이고 중첩 `-f` 출처가
비자명하므로, 전개된 정렬 목록을 소비 단계의 `-v`에서 출처(`expanded from -f vendor.f`)와
해소 베이스(CWD vs file-dir)와 함께 echo한다. 사이클은 diag 진단(`E-FLIST-CYCLE: a.f -> b.f
-> a.f`)으로 보고.

---

## 추적성 (traceability)

`diag`는 이미 span 있는 단일 진단의 file:line:col + caret를 그린다 → **compile/parse 케이스는
그대로 커버**(04:26/28: lexer가 토큰에 file/line/col 부착). vita-log가 더하는 셋:

1. **include / macro / `-f` 확장 스택(compile-time).** 중첩 `` `include ``·중첩 `-f`/`-F`에서
   출처가 비자명하므로 context Frame(miette `related()`)을 붙인다: `in file X, included from
   Y:line`, `in expansion of macro WIDTH defined at Z`, `expanded from -f vendor.f`.
2. **runtime 위치 — §7 side-table(핵심).** sim-ir는 span-free(D4)라 runtime `$fatal`/range
   위반은 builtin-call IR 노드 인덱스만 안다. vita-log가 그 인덱스를 §7 side-table
   `node_index → {file_id, byte_range}`에서 찾고, work 매니페스트의 `file_id → path`로 해소해
   SourceLoc를 복원, **diag에 넘겨** compile 에러와 동일한 caret 스니펫으로 렌더. side-table은
   **기본 포함**이며 명시적 strip(`velab --strip-locations`/release)된 경우에만 빠진다 — strip된
   스냅샷을 로드하면 vrun이 **로드 시 1회 `W-RUN-NO-LOCATIONS`**(`-Wno-`로 억제 가능)를 내고,
   진단 발생 시 위치만 `(source location unavailable; rebuild snapshot with locations)`로
   graceful degrade — 절대 crash 안 함(14 §7). runtime severity 줄은 sim_time도 prepend
   (IEEE 1800 §20.10/§20.11; sim_time은 sim-engine 출처).
3. **instance/hierarchy 경로(`%m`).** 다중구동/연결성/runtime 에러는 file:line만으론 모호하므로
   평탄화된 계층 경로(`tb.dut.u_alu`)를 Frame으로 출력(Xcelium/VCS가 elab 에러에 정확히 이걸
   찍음). span과 직교.

> **elaboration-시점 severity(신규).** `$fatal`/`$error`/`$warning`/`$info`는 IEEE상 *두* 맥락
> (elaboration, runtime)에서 발화한다(hdl-reference 13-misc). elaboration-시점 호출은
> `elaborate` 크레이트가 **같은 LogEvent::Diagnostic**으로 방출하되 **sim_time 없음**, span은
> AST에서 직접(§7 side-table 불요), 부분 instance/generate 경로를 Frame으로. elaboration
> `$fatal`은 단계 중단 + exit class 1(velab/vita 실패, staleness와 구별). 코드:
> `F-ELAB-USER-FATAL`/`E-ELAB-USER-ERROR`/`W-ELAB-USER-WARNING`/`I-ELAB-USER-INFO`.

span/caret은 diag, side-table 룩업·include/macro/-f 스택·sim_time·instance 경로는 vita-log —
역할이 깔끔히 갈린다. D4와의 연결이 명시적: sim-ir 코어는 span-free·SchemaHash-clean, 위치는
독립 버전 오버레이에 탑승.

---

## 로그 파일 (tee)

- **상시 자동 로그(Xcelium `xrun.log` 모델).** 사용자가 아무 플래그를 안 줘도 각 단계가
  **항상 로그 파일을 남긴다** — 사후 추적(특히 비결정·드문 실패)을 보장. 기본 명명은 단계별로
  달라 staged 런이 서로 덮어쓰지 않는다: `vcmp` → `vcmp.log`, `velab` → `velab.log`, `vrun` →
  `vrun.log`, `vita` → `vita.log`(통합 transcript). 위치는 cwd(또는 `--log-dir <dir>`).
- **sink 플래그(오버라이드):** `--log <file>` / `-l <file>`는 자동 명명 대신 경로를 지정
  (`-` = stderr, vvp `-l -` 패리티). `--log-dir <dir>`는 자동 로그의 디렉터리만 바꾼다.
  `--no-log`는 자동 로그를 끈다(스트리밍만). 셋 다 bucket C(§14 §6).
- **병렬 동일-단계 충돌 주의:** 한 디렉터리에서 같은 단계를 *동시에* 여러 번 돌리면(예 병렬
  `vcmp`) 기본 명명이 충돌한다. 이때는 runner/CI가 `--log <file>` 또는 `--log-dir <dir>`로
  분리한다(상시 자동 로그를 택한 대가 — 단일 빌드에선 단계별 명명으로 충분).
- **tee 의미:** 사람용 transcript(배너+진행+모든 `$display` 줄+모든 severity/진단 줄)를 단일
  이벤트 스트림에서 **터미널과 파일 둘 다**에 쓴다 — 파일은 콘솔의 충실한 replay, 별도 채널이
  아니다. 파일 레이어는 ANSI 무조건 제거.
- **append vs overwrite:** 기본 overwrite, `--log-append`(VCS `-a` 패리티)로 누적.
- **terminal-vs-file 분리:** 내용 동일, (a) ANSI 색은 터미널만, (b) `-q`는 stdout의 `$display`
  복사만 억제하고 **파일 복사는 무관** → `-q --log run.log`는 깨끗한 콘솔 + 완전한 로그.
- **항상 로깅되는 spine(억제 불가):** 배너+provenance stamp, 모든 Error/Fatal, staleness/schema
  게이트 거부, 최종 카운트 요약. CI grep 척추 — 억제 플래그는 Note/Warning/Info와 `$display`
  stdout 복사만 건드린다.
- **구조화 sink(전망):** `--diagnostics-json <file>`(같은 `{code,severity,location,message}`를
  JSON으로 — CI가 pretty 텍스트를 파싱 안 하게)은 연기. 모든 이벤트의 안정 MsgCode가 두 채널을
  잇는다.

---

## Verbosity / suppression 플래그 (전부 bucket C)

> **구현 상태:** **`-Wno-<CODE>` / `-Werror[=<CODE>]`는 구현됨**(2026-06-10, `vita-log`
> `GatePolicy`/`GatedSink` — 전 applet, 알 수 없는 mnemonic은 E0001 exit 3, Error/Fatal
> spine은 억제 불가, 승격 실패 시 산출물 미생성). **2단계도 구현됨**(2026-06-10, 전 applet):
> `-q`/`-v`/`-vv`/`--verbosity=<0..3>`(`-q`는 터미널의 `$display`+progress 복사만 억제 — 진단
> spine·`--log` 복사는 무관; `-v`는 유효 files/defines/incdirs echo; `-vv`는 표면만 예약 = 현재
> `-v`와 동일 렌더), `--log <file>`/`-l`(단일 writer tee: RTL+진단+progress를 방출 순서대로
> 한 파일에, `-`=stderr, 기본 overwrite)·`--log-append`, **counts epilogue**(`errors=E
> warnings=W notes=N`를 stage 끝 stderr에 항상 — Fatal은 errors에 합산, notes=Info+Note,
> post-gate 카운트라 `-Werror` 승격이 errors로 이동; `--help`/`--version`/usage-error엔 없음).
> 잔여 미래형: `-Wwarn=`/`--suppress=`/`--error-limit`/자동 `<stage>.log`/`--log-dir`/
> `--no-log`/색상 계열/`--diagnostics-json` — `vita --help`가 진실 공급원.

전부 bucket C다(RULE API: PreprocInputs/ElabInputs에 필드가 없어 **구조적으로 해시 진입 불가** —
`--log`나 `-Wall`이 `.vu`/`.velab`를 무효화하면 안 됨):

- **verbosity:** `-q`/`--quiet`(vvp `-q`, stdout `$display` 억제, 파일·Error는 무관),
  `-v`/`-vv`, `--verbosity=<N>`(0=quiet,1=default,2=verbose,3=trace).
- **코드별 suppress/promote(Verilator 모델):** `-Wno-<CODE>`, `-Werror[=<CODE>]`(맨몸 = 전체
  승격), `-Wwarn=<CODE>`(기본-off 재활성), `--suppress=<CODE>`(VCS alias), `--error-limit=<N>`.
  compile 진단과 RTL `$warning`이 **같은 게이트**를 지나므로 `-Werror=W-RUN-USER-WARNING`은 RTL
  수정 없이 `$warning`을 CI 실패로(GHDL `--warn-error` 선례).
- **log/color:** 로그는 **상시 자동 기록**(`<stage>.log`); `--log <file>`(경로 지정, `-`=stderr),
  `--log-dir <dir>`(자동 로그 위치), `--no-log`(자동 로그 끄기), `--log-append`, `--no-color`
  (+ `NO_COLOR` env), `--color=auto|always|never`, `--diagnostics-json`(연기).

> **inline lint 프라그마 — 주석형 확정.** 특정 구간에서만 경고를 끄려면 소스 인라인 프라그마
> `// vitamin lint_off W-ELAB-WIDTH-TRUNC … // vitamin lint_on`(구간형)을 쓴다. **주석형을
> 택한 이유:** Verilator `// verilator lint_off`와 동형이라 친숙하고, 다른 도구(iverilog/
> Verilator)가 **주석이라 무시**해 이식성이 보장되며, Verilog-2005에도 동작한다(SV attribute
> `(* *)` 불요 — 속성형은 SV 한정 + 구간 끄기 불가라 MVP 비채택). 닫는 `lint_on`을 빼먹어
> 파일 끝까지 열린 채면 **`W-LINT-UNCLOSED`** 경고를 낸다. 프라그마는 **소스에 있으므로
> 전처리 바이트를 바꿔 RULE S 바이트 해싱으로 정확히 vcmp 소스 해시에 반영된다 — 예외가
> 아니라 일관(플래그는 bucket C, 소스 프라그마는 그냥 소스)**. 단 전파 범위는
> **textually-inlined `` `include `` 안에서만**이며, `-y`/`-v`로 별도 컴파일되는 라이브러리
> 단위로는 넘어가지 않는다(별도 단위의 바이트엔 부모 프라그마가 없으므로 — RULE S sibling
> 함정과 동류의 silent hash gap을 방지).

---

## Exit 코드 (CI 계약)

CI(09 corpus-runner/차등검증)가 게이트할 수 있게 클래스를 구분한다.
**현 구현 실태: 0/1/2/3(+101 panic 관례)** — class 2는 구현됨(2026-06-10: 아티팩트
magic/schema/version 게이트 거부 = exit 2), `-Werror` 승격 실패는 class 1("승격-warning 실패").
이 절의 `-n`/`-N`/`--error-exit`/`--quiet-exit`/`--finish-at` 플래그는 Phase-1.x 미래형이다
(현행 타임아웃은 `--timeout <ticks>`, clean exit 0).

| 코드 | 의미 |
|---|---|
| 0 | clean: compile/elab/run 성공; `$finish`(또는 `--finish-at`)로 종료; Error/Fatal·승격 warning 없음 |
| 1 | user/design 실패: compile/elab Error, runtime `$fatal`, 승격-warning 실패, 또는 `-N` 하 `$stop` |
| 2 | staleness/artifact 게이트(RULE V): vrun/velab이 magic/schema/version 게이트 거부(`E-ART-FORMAT-MISMATCH`/`E-ART-SCHEMA-MISMATCH`/`E-ART-VERSION-GATE`; RULE-V composite `E-ART-STALE-UPSTREAM`은 Phase-2). **1과 구별** — CI가 RTL 디버그가 아니라 vcmp/velab 재실행임을 앎. silent 재사용 절대 없음 *(2026-06-10 구현)* |
| 3 | CLI/usage 에러(잘못된 플래그, filelist 못 찾음, `E-FLIST-CYCLE`) |
| 101 | 내부 에러/panic: `panic=unwind` + cli `catch_unwind`(03 release 근거)로 잡아 **부분 VCD flush** + diag 내부오류 리포트. 전용 코드라 09 차등검증 runner가 vitamin crash를 RTL 실패로 **절대 오인 안 함**(101 = Rust panic 관례) |

- **class 1 세분:** 같은 exit 1 안에서 compile-실패(산출물 없음) vs runtime-실패(돌았는데
  틀림)는 **always-logged 요약의 MsgCode로 구분**한다 — 09 corpus runner는 exit code 단독이
  아니라 MsgCode로 STALE/CRASH/compile-vs-run을 분류(텍스트 grep 금지).
- **`$finish` vs `$fatal`:** clean `$finish` → 0; `$fatal(n,…)` → nonzero(class 1). `n`(0/1/2)은
  종료 시 **진단 stats verbosity만** 제어(0=silent, 1=time+loc, 2=+mem/CPU; 04-simulation-control
  공유), shell code가 아니다.
- **`-n`/`-N`(§14 §6):** `-n`은 `$stop`을 clean `$finish`로(exit 0), `-N`은 `$stop` 호출 시 추가로
  exit 1(vvp 패리티) — headless CI 실패 신호.
- **opt-in:** `--error-exit`(IEEE상 `$error`는 종료 안 하므로 opt-in으로 `$error` 발화 시
  nonzero), `--quiet-exit`(exit code 보존하며 "exiting due to errors" 꼬리만 억제).
- **`--finish-at`은 별도 timeout 신호**(silent 0 아님): 강제 컷오프는 자가 `$finish`와 다른
  결과이므로, never-`$finish` 테스트벤치를 CI가 timeout 실패로 감지할 수 있게 한다.
- **n-arg / `--quiet-exit` / `-q` 우선순위:** n-arg는 exit *stats* verbosity만, `-q`/`--quiet-exit`는
  stdout 복사·꼬리만 — 어느 것도 exit 클래스를 바꾸지 않는다.

---

## RTL severity 통합

hdl-builtins/sim-engine는 `println!`하지 않고 LogEvent를 방출한다(단일 sink가 terminal+logfile에
렌더해 `$display`와 `$error`가 sim-time 순으로 인터리브):

| RTL | 레벨 | 코드 | 동작 |
|---|---|---|---|
| `$info` | Info | `I-RUN-USER-INFO` | 계속, exit 무관 |
| `$warning` | Warning | `W-RUN-USER-WARNING` | 계속; `-Werror=`로 RTL 수정 없이 CI 실패 가능 |
| `$error` | Error | `E-RUN-USER-ERROR` | 계속(IEEE: 종료 안 함); 실패 플래그; `--error-exit` opt-in 시에만 nonzero |
| `$fatal(n,…)` | Fatal | `F-RUN-FATAL` | 중단 + 묵시 `$finish`; `n`은 exit-stats verbosity; nonzero(class 1) |
| `$display`/`$write`/`$monitor`/`$strobe` | — | — | `RtlOutput`: severity·exit 없음, 같은 tee sink |
| action block 없는 assert 실패 | Error | `E-RUN-ASSERT-FAIL` | 같은 게이트(SVA Phase 2 — 코드 예약) |

**IEEE 강제 메시지 내용**(13-misc "자동 추가되는 출력 정보"): 모든 severity 줄은 severity 라벨 +
file:line(§7 side-table) + 계층 scope 경로 + (runtime) sim_time을 담는다. vita-log가 넷을 조립,
diag가 위치 본문을 렌더.

**uniform gate:** compile-time 진단과 runtime severity가 같은 suppress/promote 게이트를 지나므로
`-Wno-`/`-Werror=`/`--error-limit`이 양쪽에 균일 적용 — 한 코드 경로, RTL vs 도구 메시지 특수화
없음. 이것이 두 서브시스템이 아니라 하나인 이유다.

---

## 크레이트 배치

워크스페이스 **13 → 14** 크레이트.

- **`diag`(확장, leaf 유지):** `Severity` enum, exhaustive `MsgCode` enum(안정 `VITA-####` 태그 +
  기본 severity + 제목 + miette url()), `Frame`/related, `Diagnostic` 구조체, **그리고 `LogSink`
  trait + `LogEvent` 타입**을 추가. 전부 순수 데이터 + trait이라 IO·tracing 의존 없음 → 최하위
  leaf로 남아 모든 크레이트가 싸게 타입 진단을 만든다.
- **`vita-log`(신규):** 구체 tracing-backed `impl LogSink`. LogEvent 스트림, terminal+file sink,
  severity 라우팅, suppress/promote 게이트, 카운트, exit-code 정책, 배너/진행/filelist echo,
  §7 side-table→span 브리지를 소유. 의존: `diag`(렌더 + trait + 타입), **`vita-artifact`**(§7
  위치 side-table 오버레이 + `file_id→path` 매니페스트 맵 읽기), `sim-ir`(instance/hierarchy
  경로 타입), `tracing` + `tracing-subscriber`(순수 Rust, C 없음 — 03 준수), `miette`.
- **emitter**(`elaborate`/`hdl-builtins`/`sim-engine`/`vita-artifact`)는 `&dyn LogSink`만 받아
  **`diag`에만 의존**. **`cli`만 `vita-log`를 의존**(구체 sink 생성·설치, 네 드라이버 배너/exit).
- **비순환 점검:** `diag` leaf; `vita-artifact → {hdl-ast, sim-ir, hdl-preprocess, diag,
  vita-artifact-derive}`(vita-log로 향하는 엣지 없음); `vita-log → {diag, vita-artifact, sim-ir,
  tracing}`; `cli → 전부 + vita-log`. 사이클 없음, C 의존 없음, cargo-only.

---

## Sources

- 03-build-and-portability.md (cargo-only · `panic=unwind`+catch_unwind → exit 101)
- 04-architecture.md (diag 크레이트 역할 · 파이프라인 · 크레이트 표)
- 09-testing-and-verification.md (corpus-runner · JUnit · MsgCode assertion · exit 분류)
- 14-staged-artifacts.md (§2 RULE C/API/S · §6 CLI · §7 위치 side-table)
- hdl-reference/system-tasks/13-misc.md ($fatal/$error/$warning/$info), 04-simulation-control.md
  ($finish/$stop severity 0/1/2), 01-display-io.md; systemverilog/07-assertions-sva.md
- 상용/오픈소스 선례: Cadence `*N/*W/*E/*F`+`nchelp`, Synopsys `Error-[CODE]`, Verilator
  `-Wno-`/`-Werror-`+SARIF, Icarus vvp `-l`, GHDL `--warn-error`
- Rust: `tracing`/`tracing-subscriber`, `miette`(채택 렌더러), `codespan-reporting`(fallback). 파서/diag 결정 근거는 02.
