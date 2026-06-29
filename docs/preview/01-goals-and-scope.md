# 01 · 목표 · 범위 · 성공 기준

## 목표 (in-scope)

이하 항목이 본 프로젝트의 구현 범위다.

- **HDL 소스 전체 파이프라인**: preprocess → lex → parse → elaboration → simulation.
  각 단계는 독립 크레이트로 분리해 단위 테스트가 가능하다.
- **단계별 실행 모델**: `vita` 원샷(compile→elaborate→simulation 일괄) 외에, 단계별 명령
  `vcmp`(compile)·`velab`(elaborate, vcmp 산출물 소비)·`vrun`(simulation, velab 산출물 소비)으로
  나눠 실행할 수 있다. 단계별 독립 빌드·디버깅, 변경 없는 단계 스킵(산출물 재사용)을 지원하며,
  상용 EDA(Cadence·Synopsys)의 compile/elaborate/simulate 분리에 대응한다 (§4 아키텍처).
- **문법 검사** — parse 단계에서 수행. 오류는 소스 위치와 함께 진단 출력.
- **elaboration 단계 점검 항목** — 파라미터 해소, 계층 연결성, 타입/포트 정합, 미연결 신호, 다중구동(multiple driver) 등.
- **이벤트 구동(event-driven) 시뮬레이션 커널** + `timescale` 기반 정밀 시간 모델 (§6).
- **VCD 파형 생성 (IEEE 1364)** — RTL 내 dump 시스템 태스크 호출 시에만 활성; 자동 항상-덤프 아님.
  CLI 편의 플래그(`vrun --force-dump`)는 후속 옵션으로만 검토하며, 기본은 RTL이 주도한다.
- **표준 Verilog/SystemVerilog system tasks/functions (`$`로 시작) 전수 지원** — display · I/O · 파일 I/O · 메모리 로드 · 시뮬레이션 제어 · 시간 · 변환 · 비트벡터 · 수학 · random · VCD dump · assertion 샘플링 · introspection 등 전 범주.
  구체 목록과 Phase별 커버리지는 `hdl-reference/system-tasks/00-index.md` 참조.
- **3개 HDL 지원** (로드맵 단계별): SystemVerilog(IEEE 1800) → 그 부분집합인 Verilog(IEEE 1364) → VHDL(IEEE 1076). 단계별 상세는 [로드맵 섹션](#phase-1-mvp-정의) 참조.
- **3-OS 소스 빌드** (cargo) + CI 매트릭스.
- **멀티 라이브러리** *(✅ v1 구현 2026-06-12: `vcmp --work`/`velab -L`/`--top` + vrun RULE-V 자동 게이트 — doc-14 §3 구현 상태 참조; `--lib-map`/`-y`/`-v`는 후속)* — 단위를 `library:unit` 논리 키로 주소화하고 논리명→디렉터리 매핑(`cds.lib`/`synopsys_sim.setup` 계열)을 지원 (D3, §14).
- **단계 산출물 온디스크 포맷 + staleness 재검증** — `vcmp`의 `work/` 라이브러리 + `velab`의 `.velab` 스냅샷을 해시 결합으로 묶어, 상류가 바뀐 stale 산출물에 대한 `vrun`을 거부한다 (RULE V, §14). *(✅ 전 체인 가동 2026-06-12: schema_hash+format_version 게이트 + lib-모드 `.velab`의 WorkConsumed 기록(매니페스트/blob/소스·include 다이제스트)을 bare `vrun`이 자동 재해시 — 불일치=`E-ART-STALE-UPSTREAM` exit 2. 명시 경로는 `vrun --upstream`)*
- **filelist** — `-f`/`-F` 재귀 중첩 + `+incdir+`/`+define+` 집계 *(2026-06-10 전부 구현: argv-레벨 전개, 사이클/깊이/glob/env 가드, MIXED-BASE/OVERRIDE/WRONG-STAGE, -D/-I → PreOpts)* (§14 §3.1).
- **진단/로깅 서브시스템** — transcript + 로그파일 tee, severity 라우팅, 소스 위치 추적 (§13). *(현 구현: stderr 진단+소스 위치; tee/라우팅 게이트는 `vita-log`와 함께 Phase-1.x)*
- **에러 코드 카탈로그** — 안정 `MsgCode`(mnemonic + `VITA-####`) CI 1:1 동기 + `vita explain <CODE>`(doc-15 항목 출력, mnemonic/번호 양형 — 2026-06-10 구현) (§15).

## 비목표 (out-of-scope, 현 단계)

현 설계 단계에서 명시적으로 범위 밖으로 지정한 항목들이다.

- **합성(synthesis) 툴 자체 구현** — 단, 참조 문서에는 각 구문의 합성 가능 여부를 명기한다.
- **컴파일드(네이티브/JIT) 시뮬레이션 백엔드** — IR 경계만 열어두고 후속 단계로 미룬다. (`sim-ir` 설계는 이 확장을 고려해 언어 중립으로 유지된다.) *(2026-06: Stage C 바이트코드 VM=C1·C2 가동 + native-eval C4-lite 착수해 식-바운드 VM ~0.42x 달성 — P5 차분 게이트가 인터프리터와 byte동일 강제; signed>64·>128bit·sysfunc·real native lane은 잔여 저-ROI)*
- **커버리지/UVM** (N5+ — 단, functional coverage 코어는 ✅ 구현됨, N5/N5-G) · **`program`·`union`** 및 **class 잔여(N7-REST: randomization·parameterized class·static/local 멤버 등)**(*`class` 코어+단일 상속+virtual 동적 디스패치는 ✅ 구현됨, N7*) · **math transcendentals**(`$ln`/`$log10`/`$exp`/`$sqrt`/삼각 — N6, pure-Rust libm 3-OS 결정성 핀 대기로 loud).
- **파형 GUI 뷰어** — VCD 출력을 GTKWave · Surfer 등 외부 뷰어로 확인한다.
- **FST 등 VCD 외 파형 포맷** — 후속 확장으로만 기록.
- **UPF/전력, SDF 타이밍 백애너테이션, DPI-C, 커버리지/UVM** 등 고급 검증 기능.

## 성공 기준 (측정 가능)

아래 항목을 모두 통과하는 것이 Phase 1 완료 조건이다.

- 대표 RTL 테스트벤치를 **Icarus Verilog(`iverilog` + `vvp`)를 golden으로 차등검증**했을 때 신호값과 천이 시각이 일치한다. Verilator는 **보정된(calibrated) 부분집합**에서 비교한다 — 2-state·X-init·조합 `$display` 등 비-IEEE 차이는 `known_quirks`로 carve-out (§9). 도구 충돌 시 IEEE LRM이 최종 권위.
- 생성 VCD가 표준 뷰어(GTKWave 등)에서 오류 없이 로드되고, **golden VCD와 정규화 diff가 일치**한다. (식별자 코드 차이를 흡수하는 정규화기 포함)
- 동일 소스가 **Ubuntu · RHEL · macOS에서 동일 결과**로 빌드·실행된다.
- `timescale`이 다른 모듈이 혼재해도 전역 시간축이 어긋나지 않는다 — 64-bit 정수 시간 + precision 환산 정밀도 테스트 통과 (§6.3).
- **표준 system tasks/functions 컴플라이언스 코퍼스** (범주별 최소 1개 케이스) 전수 통과.
- **staleness 재검증이 동작한다** — 상류 소스가 바뀐 stale `.velab` 스냅샷에 대한 `vrun`은 거부된다(`E-ART-STALE-UPSTREAM`, exit class 2 — RULE V). 이 동작을 최소 1회 테스트한다(mtime이 아닌 해시 기반).
- **진단 코퍼스는 메시지 텍스트가 아니라 `MsgCode`로 assert**하며(`expect_codes`, §9), 모든 코드가 §15 카탈로그와 1:1 동기(CI 게이트)다.

## 타깃 환경

| OS | 버전 |
|---|---|
| Ubuntu | LTS |
| RHEL | 8 / 9 계열 |
| macOS | Apple Silicon + Intel |

빌드 철학: 원문 소스 → 각 OS에서 `cargo build`. 사전 빌드 바이너리 배포에 의존하지 않는다.

**순수 Rust 코어 + 최소/제로 C 의존성.** 외부 C 라이브러리 의존을 피해 3-OS 빌드 마찰을 제거한다.

**MSRV(최소 지원 Rust 버전) 고정** + `rust-toolchain.toml`로 재현성 확보.

## Phase 1 (MVP) 정의

Phase 1의 범위는 **SystemVerilog 합성가능 RTL 서브셋** — Verilog-2005 RTL 전부를 포함한다.

**파이프라인:** preprocess → lex → parse → elaborate → event-driven sim → VCD

**백엔드:** 인터프리터 방식 (IR-walking). 정확성 · VCD · timescale 정밀도를 먼저 확보한 뒤, 후속 단계에서 컴파일드 백엔드를 `sim-ir` 경계 너머에 추가한다.

**Phase 1 구문 동결 (IN-MVP / deferred):** Phase 1 경계는 합성가능성 범례가 아니라 아래 표가 단일 기준으로 정의한다.

| 분류 | IN-MVP (Phase 1) | deferred (Phase 2+) |
|---|---|---|
| 설계 단위 | `module`/`endmodule`, 포트, `parameter`/`localparam`, `generate`/`genvar`, `interface`/`modport`(*2026-06-11 IN 승격*), **`package`/`import`/`pkg::`**(*2026-06-12 IN 승격, v7 — param/enum-label/func/task 평탄화; 패키지 변수·패키지내 import는 loud*), **계층 이름 참조 read+write**(*✅ 구현 N3/N3.1 + HIER-REST 완결 — `tb.dut.x`·`dut.mem[i]`·`dut.grid[i][j]`·packed `dut.pm[i]` 식-내 read **및 계층 WRITE `dut.x = v`**·element/bit/part-select·indexed-segment(`g[0].x`); 잔여 컷=loud: interface-member array element·`$dumpvars` hier arg 등*), **`class` 코어+단일 상속+virtual**(*✅ 구현 N7 — class·필드·new/ctor·this·handle/null·extends/super·virtual 동적 디스패치 vtable*) | `program`, **class 잔여(N7-REST: randomization·`program`·parameterized class·static/local 멤버)** |
| 자료형 | `wire`/`reg`/`logic`/`integer`, 벡터·packed array, `enum`/`typedef`/packed `struct`, 동적 배열·queue(`[$:N]` bounded 포함)·연관 배열(int/string 키)(*2026-06-11 IN 승격, format_version 5/6*), **full `string` 타입**(*2026-06-12 IN 승격, v7 — 힙 핸들·len/getc/putc/substr/toupper/tolower/compare·StrCmp 비교·`$sformat(f)`; concat·decl-init·port는 loud*) | `union` |
| 절차 블록 | `initial`, `always`, `always_ff`/`always_comb`/`always_latch`, fork-join + **`join_any`/`join_none`/`disable fork`**(*P2-E 2026-06-12*)·**`wait fork`**(*v8 2026-06-14 — 직계 자식 배리어*), **`final`**(*P2-E — 타이밍 컨트롤=loud*), named `event`/`->`/`@(ev)`, **automatic/recursive 함수·태스크(frame-call)**(*✅ 구현 2026-06-17 B-track — frame-local=진짜 net+런타임 라우팅, 함수=`run_frame_call`/태스크=`run_task` executor, disable-unwind, per-decl `automatic` lifetime override; 재귀=`MAX_CALL_DEPTH=8192` 가드; 잔여 컷: cross-frame disable·block-local `automatic` 폼2·staged-vrun 사이드카 직렬화 미지원=one-shot `vita`만*) | — |
| 문장 | blocking `=` / nonblocking `<=`, `if`/`case`/`casez`/`casex`, `for`/`while`/`repeat`/`forever`/**`do-while`**(*P2-E 파스 desugar*), `begin`/`end`, `foreach`, `disable`(동봉 named block + `disable fork`), proc-`assign`/`deassign`, immediate `assert`, **concurrent `assert property`**(*Phase-3 SVA 서브셋, 2026-06-14~16 — 단일/다중-클럭 `\|->`·`\|=>`, 시퀀스 `##n`/`##[m:n]`/`##[m:$]`/`[*n]`/`[*m:n]`/`throughout`/`[->n]`/`[=n]`/`within`, named property/sequence+formal args, generate-scope; 전부 순수 IR-0 desugar·iverilog가 SVA 거부→hand-IEEE 핀*), **`unique`/`priority` if/case**(*P2-E — no-match=런타임 warning; 다중-match 검사는 컷*) | multi-term 시퀀스 cross-clock·sequence local var·recursive property |
| 타이밍 | `#delay`(상수+**런타임 식** — format_version 4), `@(event)`, `wait`(테스트벤치), intra-assignment `= #d`(실semantics)·`<= #d`(transport, v5)·**repeat-event blocking `= [repeat(n)]@(ev)`**(*`cfef719` — iverilog 차분*)·**repeat-event NBA `<= [repeat(n)]@(ev)`**(*✅ 구현 2026-06-17 N1 `f4aab23` — capture-now/`fork…join_none`/NBA-write desugar, `repeat(0)`=plain NBA, 순수 IR-0; X/Z 상수 count=0회·동시-틱 region-tie=LRM-faithful pin*) | clocking block(N4=조건부 NO-GO — Preponed/sampled-value 스케줄링 리전 부재) |
| 연속 대입 | `assign`(+지연), `force`/`release`(연속 재평가 포함) | — |
| system tasks | 아래 핵심 셋 + conversion(`$signed`/`$unsigned`/`$rtoi`/`$itor`/`$realtobits`/`$bitstoreal`)·`$clog2` + **v7 일괄 승격(2026-06-12)**: 파일 I/O(`$fopen/$fclose/$fdisplay/$fwrite`+b/o/h) · `$readmemb/h` · `$random`(Annex N)/`$urandom(_range)`(자체 계약) · `$stime` · `$test$plusargs`/`$value$plusargs` · bit-vector(`$bits/$countones/$onehot(0)/$isunknown`) · `$sformat(f)` · **assertion 샘플링 `$past`/`$rose`/`$fell`/`$stable`**(*Phase-3 SVA — prev-reg desugar, action block 내부 포함; iverilog 거부→hand-IEEE*) + **v9 일괄 승격(2026-06-18)**: 파일 READ(`$fread/$fscanf/$fgets/$sscanf/$feof/$fgetc`) · `$writememb/h` · `$countbits` · introspection(`$typename/$cast/$size/$left/$right/$low/$high/$increment/$dimensions/$isunbounded`) · `$changed`/`$sampled` · `$exit`(=`$finish` 별칭) · `$monitoron/off` · `$dist_uniform` | math transcendentals(libm 3-OS 리스크)·비-uniform `$dist_*`·`$system`·`$assertcontrol` (플랜 = `docs/ROADMAP.md` §4) |

> **합성가능성 마커는 구현 경계가 아니다.** `hdl-reference/`의 합성가능성 범례는 RTL→게이트 *합성* 가능 여부를 표기할 뿐이다. `initial`·`#delay`·`$display`·`$finish`는 합성 불가로 표기되지만 **시뮬레이터에 필수**이므로 Phase 1 IN이다. MVP의 IN/OUT 경계는 위 동결 표가 단일 기준이다.

**system tasks 핵심 셋 (Phase 1에서 반드시 지원):**

| 범주 | system tasks |
|---|---|
| 출력 (display I/O) | `$display` / `$write` / `$monitor` / `$strobe` |
| 시간 | `$time` / `$realtime` |
| 시뮬레이션 제어 | `$finish` / `$stop` |
| VCD dump 패밀리 | `$dumpfile` / `$dumpvars` / `$dumpon` / `$dumpoff` / `$dumpall` / `$dumpflush` / `$dumplimit` |

각 system task의 전체 시맨틱과 인자 명세는 `hdl-reference/system-tasks/` 섹터 참조.

Phase 2(SV 확장) 이후의 system tasks — 파일 I/O, 메모리 로드, 변환, 비트벡터, 수학, random, assertion 샘플링, introspection 등 — 는 `hdl-reference/system-tasks/00-index.md`의 Phase별 커버리지 매트릭스에 명시된다.

## 알려진 v1 단순화 (IN-MVP이되 의도적 한계 — 결함 아님)

아래는 Phase-1 IN 기능이지만 **의도적으로 단순화**한 동작이다. 모두 결정적·문서화됨이며, 정밀화는 Phase-1.x/Phase-2에서. (구현 검증 중 확인된 항목; 상세는 저장소 `docs/REMAINING_WORK.md`.)

| 영역 | v1 동작 | 정밀 동작(향후) |
|---|---|---|
| `casez`/`casex` 와일드카드 | scrutinee·label의 **모든** x/z를 don't-care로 마스킹(`reduction_or(scrut^label)!==1`) | `casez`는 z/?만, `casex`는 x/z만 (explicit-x-in-casez 분리) |
| 배열/벡터 인덱스 범위초과 | 읽기 all-X / 쓰기 무시(클램프 아님) + E-RUN-RANGE(VITA-E4002) 진단 발행(rate-limit) | `E-RUN-RANGE`(VITA-E4002) 런타임 진단 발행(엔진 diag-sink 도입 시) |
| unpacked/packed *서브차원* 인덱스 초과 | ✅ **per-dim bounds 구현(2026-06-12)** — 다차원(d≥2) 인덱스마다 `lo≤idx≤hi` 가드를 IR에 합성(1-D는 기존 wrap+flat 검사로 이미 정밀 → byte-불변). 위반 시 unpacked=read X/write no-op+E4002, packed bit-space=silent X(벡터 part-select 계약 일치). **이전: 평탄공간 alias(silent-wrong, `g[0][5]`→`g[1][2]`); ⚠️ iverilog 13.0도 동일하게 alias → hand-IEEE §7.4.6 핀 레인** | (완료) |
| 다차원 unpacked 배열 부분 슬라이스 / whole-array | ✅ **배열 대입 구현(2026-06-12)** — `a = b;`·`g[i] = row;`·`<=`(+transport `#d`)를 IEEE §7.6 위치 대응(선언순 좌→좌, 방향 반영)으로 element-wise 전개(elaborate desugar, IR 0). 대입 외 컨텍스트의 whole-array는 **loud E3009로 승격**(이전: silent word-0 읽기/쓰기; `$dump*` 인자만 word-0 현상 유지=item ⑤ 영역). iverilog 13.0이 기능 자체 미지원 → hand-IEEE 핀 레인 | (완료 — 잔여 컷, 전부 loud: `= #d` intra-delay·연속 assign·배열 비교/포트 통과·`'{}` 패턴·슬라이스 index가 타깃 배열을 읽는 별칭·4096 원소 초과 전개) |
| instance array (`dff u[3:0](...)`) | ✅ **언롤 구현(2026-06-12)** — 선언순 첫 인덱스=MSB 청크(iverilog 핀), W=P 공유/W=N·P 슬라이스, ANSI 자식+상수 범위+식별자 conn(그 외 loud) | (완료 — 잔여 컷: non-ANSI 자식·복합 conn 식) |
| X/Z 인덱스 | 읽기 all-X / 쓰기 no-op | 동일(이미 정밀) |
| `$time`/`$realtime` 멀티-timescale | per-process 단위로 정확 | 동일(이미 정밀) |
| `assign #d` 지연 | ✅ **inertial 모델 구현(2026-06-12)** — RHS 재변경이 pending write를 세대 카운터로 무효화(d보다 좁은 펄스=필터, 정확히 d 펄스=생존 — iverilog 차분 핀). NBA `<= #d`는 transport 유지(IEEE 의도 그대로) | (완료) |
| `$stop` | 배치 종료(에서 `$finish`와 별개 exit class) | 대화형 브레이크포인트는 비지원(배치 시뮬레이터) |
| `$dumpvars`/`$dump*` 배열·인자 | ✅ **per-element + depth/scope 구현(2026-06-12)** — 배열은 원소당 `$var`(`mem[4]`·`g[1][2]`, dims는 `.velab` 10번째 trailer 사이드카), `$dumpvars(N, scope/net…)`의 depth(N=0 전체/N=레벨 수, iverilog 핀)·scope(elaborate가 `fq\x01raw` 2-후보 string const로 인코딩)·net/배열 인자 선별. 추가 `$dumpvars` 호출=W4021 1회+무시(합집합=컷), 미해소 scope=전체 폴백, 레벨-only 폼=전체. ⚠️ iverilog는 memory 인자 자체를 에러 — 배열 인자 수용은 hand-확장 | (완료 — 잔여 컷: 다중 호출 합집합·`[0:0]` 배열은 단일 var) |
| `>128bit` unsigned / `>64bit` signed 산술 | ✅ **full multi-word 구현(2026-06-12)** — add/sub/mul(school)/div·mod(short/restoring, 부호=IEEE: 몫 절삭·나머지=피제수 부호)/pow(square-multiply, 음수 지수=IEEE 표) word-grid mod 2^w + `%d` 십진 렌더 임의 폭(이전: signed>64=unsigned 표시·>128=silent 절단). 전부 iverilog 차분 핀 | (완료) |
| LOCAL array-of-packed 서브원소 r/w (`qm[i][j]`) | silent 1-bit(packed 바이트여야 함) — `lower_array_read`/`collect_array_write` 양쪽 pre-existing(계층 read 슬라이스 N3.1-follow-on에선 loud-reject로 회귀 차단) | 정밀화 = N3.2 전용 슬라이스(IR-0, iverilog 차분; loud-reject로 분리 핀) |
| `force`/`release` | **sample-once**(RHS를 실행 시점 1회 평가 — iverilog 오라클과 동일 모델·동일 경고 의미), whole-net/var 타깃만(bit/part-select=loud reject) | IEEE full 절차적 연속 대입(RHS 피연산자 변화 시 재평가) |
| `foreach`(고정크기 unpacked 배열) | **loud-reject**(E3009) — 2026-06-22 감사 정정: dyn array/queue/assoc에서는 동작하나, 고정크기 `int a[0:N]`에 대한 `foreach`는 v6 uniform `.first/.next` desugar가 합성하는 메서드 호출을 `inline_function`이 거부(`E3009 hierarchical function call`)해 깨진다. 동작은 인덱스 `for` 루프로 대체. **이전 freeze 표는 `foreach`를 무조건 IN으로 과장 표기했었다** | 고정크기 경로를 plain 인덱스 walk로 desugar(REMAINING_WORK) |
| 자유함수/모듈/패키지 함수의 `return` 키워드 | **loud-reject**(E3009 `return outside ...`) — 2026-06-22 감사 정정: `return`은 현재 **class 메서드 본문에서만** 동작하고, 자유·모듈·패키지 함수는 name-assign(`f = expr`)으로 반환해야 한다(프레임-함수 lowering이 `cur_return`을 설정하지 않음). loud이므로 silent-wrong 아님 | 프레임-함수에 exit-block+`cur_return` 부여(**golden 형상 flip → format_version bump 동반**, REMAINING_WORK) |
| `function void` 반환형 · typed `parameter int W` | **loud-reject**(E2002 파스) — `void`는 렉서 키워드가 아니라 메서드명으로 소비됨(void 메서드는 `task`로 작성). `parameter int W=32`처럼 type-키워드를 둔 파라미터도 미파싱(untyped `parameter W=32`는 동작). module/package/interface 공통 | `void` 키워드 + typed-param 파서 확장(REMAINING_WORK) |

## Sources

- 본 spec §2.1 · §2.2 · §2.3 · §3 · §9 — `docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`
