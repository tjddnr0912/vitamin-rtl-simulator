# 15 · 에러 코드 레퍼런스

> vitamin이 내는 **모든 진단 메시지 코드의 원인·예시·해결을 정의하는 단일 권위 레퍼런스**다.
> `vita explain <CODE>`의 소스이며, **vitamin 구현의 산출물(deliverable)** 로서 코드가
> 추가·변경될 때마다 본 문서도 함께 갱신된다(아래 거버넌스).

---

## 거버넌스 (이 문서의 규칙)

- **이 문서는 코드에서 자동 생성되지 않는다.** 사람이 작성하는 원인 설명 산출물이며, `diag`
  크레이트의 `MsgCode` exhaustive enum과 **1:1 동기**를 유지한다.
- **CI 동기 게이트:** CI는 `MsgCode` enum의 모든 variant가 본 문서에 항목을 가지며, 본 문서의
  모든 코드가 enum에 존재함을 검증한다 — **새 에러를 추가하면 본 문서 항목 추가 없이는 빌드가
  통과하지 못한다.** 이것이 "에러 추가 시 문서 동기 갱신"을 강제하는 메커니즘이다.
  - **게이트 대상 = `MsgCode` enum에 실재하는 코드 = 본문 §0–9의 full-entry 코드뿐이다.** 부록 A의
    예약(미구현) 코드는 아직 enum에 등재되지 않았으므로 **bijection 게이트에서 제외**한다. 예약 코드가
    본문 형식으로 승격될 때 enum variant 추가와 **동시에** 게이트 대상이 된다(승격이 곧 enum 등재).
- **mnemonic이 1차 안정 식별자다**(예 `E-ELAB-MULTIDRIVER`) — 의미로 고정, **renumber 불가**.
  CLI(`-Wno-`/`-Werror=`)·corpus(`expect_codes`)·문서는 **숫자가 아닌 mnemonic으로 참조**한다.
- **`VITA-<S>####` 숫자는 보조**(빠른 grep용). 한 번 부여하면 **영구**(빈 번호는 빈 채로,
  재사용·renumber 금지 — rustc `E0001` 방식). severity 접두 `S` ∈ {E=Error, W=Warning,
  I=Info, F=Fatal}. **초기 36개**(시드 할당)는 카테고리 내 mnemonic 알파벳순으로 부여했고(현재 본문
  full-entry 코드는 **55개** — 2026-06-12 v7 기준), **이후 추가는
  해당 밴드의 다음 빈 번호를 단조 부여**한다(알파벳 재정렬 금지). 공식 출처 기반 추가 케이스
  인벤토리는 **부록 A** 참조(미구현 예약 코드 107개 + 밴드 5/6/7).
- severity·게이트·exit 의미는 [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md).
- **억제/승격 플래그는 구현됨(2026-06-10):** `-Wno-<MNEMONIC>`(Warning/Info 억제; Error/Fatal
  spine은 불가)·`-Werror`(전체)·`-Werror=<MNEMONIC>`(코드별 승격, 원 코드번호 유지·exit class 1)이
  전 applet에서 동작. 알 수 없는 mnemonic = E0001 usage error(오타는 loud). 인라인 `lint_off`
  프라그마·`-Wwarn=`/`--suppress=` alias는 Phase-1.x 잔여.
- **본문 등재 = 발행 보장이 아니다(예약 dead codes):** 다음 코드는 enum에 실재하나 **현 구현에
  emitter가 0개**인 의도적 예약 상태다 — `W-PARSE-IMPLICIT-NET`(vitamin v1은 implicit net을
  생성하지 않음 — 미선언 참조는 E3010 hard error; 추후 IEEE 기본 동작 구현 시 활성화),
  `E/F/I/W-ELAB-USER-*`(elaboration-time severity 태스크 — 현재 severity 태스크는 런타임
  4xxx로 발행), `E-RUN-ASSERT-FAIL`(assert는 Phase-1.x), `W-RUN-NO-LOCATIONS`,
  `W-LINT-UNCLOSED`(lint_off 프라그마 미구현), `W-ELAB-WIDTH-TRUNC`(실제 폭-절단 경고
  emitter 구현 전까지 예약).

### 번호대 예약

| 번호대 | 카테고리 | 단계 |
|---|---|---|
| `0xxx` | GENERAL / SYSTEM | CLI·usage·error-limit |
| `1xxx` | PREPROCESS | 전처리(`include`/매크로/lint 프라그마) |
| `2xxx` | PARSE | 어휘·구문·설계단위 |
| `3xxx` | ELABORATE | 파라미터 해소·계층·연결성·elaboration severity |
| `4xxx` | RUNTIME | 시뮬레이션·RTL severity 태스크·assert |
| `5xxx` | ASSERTION / SVA | 동시 assert·assume·cover·unique 위반 (예약 — SVA **기능**은 Phase-3 구현됐으나 실패는 합성 체커의 E4003로 방출, 5xxx enum 등재는 미발화 예약. assume/cover/SVA 전용 코드는 부록 A 인벤토리 보유) |
| `6xxx` | SV-TYPE | enum·const·$cast·class 등 SV 데이터타입 (예약 — 부록 A. 단 enum/typedef/packed struct·dynamic array/queue/assoc·string은 이미 **구현됨**(별도 6xxx 코드 미발화); class/OOP·$cast는 미구현 N7) |
| `7xxx` | VHDL | VHDL 진단 (예약, Phase 3 — 부록 A) |
| `8xxx` | FILELIST | `.f` 전개(`-f`/`-F`) |
| `9xxx` | ARTIFACT | 산출물 staleness·버전 게이트 |

---

## 0xxx · GENERAL / SYSTEM

### VITA-E0001 · `E-CLI-BAD-FLAG` (Error)
**알 수 없거나 잘못된 명령줄 플래그/값.** 인자 파서가 모르는 플래그, 형식·범위가 틀린 값,
또는 해당 단계가 받지 않는 플래그를 만났을 때. 오타 플래그(`--timescal`)가 silently 무효가 되어
미묘하게 틀린 시뮬을 내는 것을 막기 위해 컴파일 전에 큰 소리로 실패한다.
```
$ vcmp --timescal 1ns/1ps rtl/top.sv
error[VITA-E0001] E-CLI-BAD-FLAG: unknown flag '--timescal' (did you mean '--timescale'?)
```
**해결:** §6 CLI 표면대로 철자·값을 고치거나 받는 단계로 옮긴다. 억제 불가(exit class 3).

### VITA-F0002 · `F-LIMIT-ERRORS` (Fatal)
**에러 한도(`--error-limit N`) 도달 — 단계 중단.** Error 누적이 임계(Verilator 기본 50;
warning 미포함)에 도달하면, 깨진 파일이 수천 줄 cascade를 뱉지 않게 단계를 즉시 중단한다.
개별 Error는 기록-후-계속이지만 한도 도달은 그 자체가 Fatal이다.
```
$ vcmp broken.sv
error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: ...   (×50)
fatal[VITA-F0002] F-LIMIT-ERRORS: error limit reached (50); aborting compile
  errors=50 warnings=3 notes=0
```
**해결:** 가장 앞 에러부터 고친다(뒤는 cascade인 경우 많음). `--error-limit <N>`으로 조정.
억제 불가(exit class 1).

---

## 1xxx · PREPROCESS

### VITA-E1001 · `E-PP-INCLUDE-NOT-FOUND` (Error)
**`` `include `` 대상 파일을 검색 경로에서 못 찾음.** 현재 파일 디렉터리 + 모든
`+incdir+`/`-I`(RULE A 입력)를 뒤져도 없을 때. 텍스트 치환은 전처리 시점 동작이라 대상이
없으면 붙일 바이트가 없다(IEEE 1364 §19.5 / 1800 §22.4). include 스택 Frame으로 출처 표기.
```
`include "defs/config.svh"        // 어느 +incdir+ 아래에도 없음
$ vcmp top.sv +incdir+./rtl
error[VITA-E1001]: `include "defs/config.svh" not found on search path
```
**해결:** `+incdir+<dir>`로 헤더 디렉터리를 추가하거나 경로/파일명(대소문자 포함 — 경로는 모든
OS에서 대소문자 구분)을 고친다. `.f` 트리 안이면 `--dump-filelist`로 확인. 억제 불가.

### VITA-E1002 · `E-PP-MACRO-ARITY` (Error)
**함수형 매크로를 잘못된 인자 개수로 호출.** `` `define NAME(a,b,…) `` 매크로를 형식 인자
개수와 다른 실인자로 전개할 때(IEEE 1364 §19.3.1 / 1800 §22.5.1). 호출 위치와 정의 위치를
둘 다 Frame으로 첨부.
```
`define MAX(a, b) ((a) > (b) ? (a) : (b))
assign y = `MAX(x);               // arity 1, 2 기대
error[VITA-E1002]: macro `MAX expects 2 arguments, got 1
```
**해결:** 형식 인자 수에 맞춰 호출하거나 `` `define `` 을 고친다. (객체형 매크로 뒤의 `(`는
호출이 아니라 리터럴 텍스트 — arity 검사 대상 아님.) 억제 불가.

### VITA-W1003 · `W-LINT-UNCLOSED` (Warning)
**inline `// vitamin lint_off` 프라그마가 닫히지 않고 EOF 도달.** `lint_off <CODE>` 구간이
짝 `lint_on` 없이 파일 끝(또는 textually-inlined `` `include `` 범위 끝)에 도달하면, 나머지
전체의 진단이 silently 억제되는 편집 실수일 가능성이 크므로 표면화한다.
```
// vitamin lint_off W-PARSE-IMPLICIT-NET
assign w = a & b;
// ... EOF, 짝 lint_on 없음 ...
warning[VITA-W1003]: 'lint_off W-PARSE-IMPLICIT-NET' at alu.sv:2 never closed before EOF
```
**해결:** 의도한 끝에 `// vitamin lint_on <CODE>`를 추가. 전파는 textually-inlined
`` `include ``만, `-y`/`-v` 라이브러리 단위로는 안 넘어감. `-Wno-W-LINT-UNCLOSED`로 억제.

### VITA-E1004 · `E-PP-RECURSIVE-MACRO` (Error)

텍스트 매크로가 자기 자신의 확장 도중 다시 호출되어 무한 확장에 빠졌다. 전처리기는 활성 확장 집합(active-expansion set)으로 이를 감지하고 해당 사용을 리터럴로 남긴 뒤 이 에러를 보고한다.

```verilog
`define A `A
`A    // E-PP-RECURSIVE-MACRO
```

해결: 매크로 본문에서 자기 참조를 제거하거나, 재귀 대신 충분히 펼친 형태로 정의한다.

### VITA-E1005 · `E-PP-RECURSIVE-INCLUDE` (Error)

`include 체인이 순환하여 이미 열려 있는 파일을 다시 포함하려 했다. canonical 경로 스택으로 감지하고 재포함을 건너뛴다.

```verilog
// a.svh
`include "b.svh"
// b.svh
`include "a.svh"   // E-PP-RECURSIVE-INCLUDE
```

해결: 순환 포함을 제거하거나 include guard(`ifndef/`define/`endif)를 사용한다.

### VITA-E1013 · `E-PP-BAD-DIRECTIVE` (Error)

알 수 없는 컴파일러 지시어, 미정의 매크로 사용, 떠돌이 backtick, 범위 밖 지시어 형태, 불균형/중복 조건부, 비리터럴 include 인자, 지시어 이름에 대한 `undef 등 전처리 형식 오류 전반을 포괄한다.

```verilog
`frobnicate        // E-PP-BAD-DIRECTIVE (unknown directive)
`UNDEFINED_MACRO   // E-PP-BAD-DIRECTIVE (undefined macro use)
`endif             // E-PP-BAD-DIRECTIVE (no open conditional)
```

해결: 지시어 철자를 확인하거나, 매크로를 먼저 `define 하거나, 조건부 블록의 짝을 맞춘다.

### VITA-W1007 · `W-PP-MACRO-REDEFINED` (Warning)

`define가 기존 매크로를 다른 본문/파라미터로 재정의했다. 새 정의가 적용되며 경고만 발생한다(동일 본문 재정의는 무경고).

```verilog
`define W 1
`define W 2   // W-PP-MACRO-REDEFINED
```

해결: 의도된 재정의가 아니면 매크로 이름을 분리하거나 `undef 후 재정의한다.

### VITA-W1008 · `W-PP-UNDEF-UNDEFINED` (Warning)

`undef가 현재 정의되지 않은 이름을 대상으로 했다. 동작은 무해하며 경고만 발생한다.

```verilog
`undef NEVER_DEFINED   // W-PP-UNDEF-UNDEFINED
```

해결: 대상 매크로 이름의 철자를 확인하거나, 정의 이후에만 `undef 한다.

### VITA-W1017 · `W-PP-TIMESCALE-DEFAULT` (Warning)

설계의 어떤 모듈도 `` `timescale ``을 선언하지 않았고 `--timescale` 플래그도 없다. 전역 시간 단위/정밀도를 기저값 `1ns/1ns`로 잠그고 경고만 발생한다(§08). 기저값은 OS·컴파일 순서와 무관한 상수이므로 결정성을 깨지 않는다.

```verilog
module top;            // `timescale 없음 → W-PP-TIMESCALE-DEFAULT
  initial #2.5 $finish; // 2.5ns → 1ns 입도 반올림 = 3ns
endmodule
```

해결: 의도한 단위/정밀도를 `` `timescale 1ns/1ps `` 형태로 파일 상단에 선언한다(부분 지정은 별개 코드 `W-PARSE-TIMESCALE-PARTIAL`/`E-PP-TIMESCALE-PARTIAL`).

---

## 2xxx · PARSE

### VITA-E2001 · `E-DUP-UNIT` (Error)
**설계 단위(module/package) 재정의.** 같은 단위 이름이 분석 소스에 두 번 이상 정의될 때(예
filelist에 소스 파일 중복, 또는 두 파일이 같은 `module m` 선언). 논리 라이브러리는 한
`library:unit` 키에 두 단위를 못 담는다. **소스는 기본 dedup하지 않는다**(§3.1 BLOCKER) — sticky
디렉티브 상속(RULE S) 때문에 두 occurrence는 다른 컨텍스트라 같은 입력이 아니므로 silent dedup이
위험하다.
```
# build.f 가 adder.sv 를 두 번 나열 (또는 두 파일이 module adder 선언)
error[VITA-E2001]: design unit 'adder' redefined
  note: first defined at adder.sv:1   note: redefined (second occurrence)
```
**해결:** 중복 소스 항목 제거 또는 한쪽 단위 개명. `--dump-filelist`로 평탄화 순서 확인.
(같은 canonical 경로가 *다른* 상속 컨텍스트로 두 번이면 `E-FLIST-DUP-CTX-CONFLICT`.) 억제 불가.

### VITA-E2002 · `E-PARSE-UNEXPECTED-TOKEN` (Error)
**문법에 맞지 않는 예기치 못한 토큰.** 어느 valid production도 이어갈 수 없는 토큰(누락 `;`,
잘못된 키워드, 불균형 `begin`/`end`, 잘못된 식). parse는 마지막 언어 의존 단계이며 토큰에
file/line/col이 붙어 diag가 caret로 밑줄. `--error-limit` 도달 시 `F-LIMIT-ERRORS`로 중단.
```
module m;
  assign y = a &        // 우변 누락 + ';' 누락
endmodule
error[VITA-E2002]: unexpected token 'endmodule', expected expression  --> m.sv:3:1
```
**해결:** 해당 위치 문법을 고친다. `--std`/`-g<year>`/`-sv` dialect가 파일과 맞는지 확인(2005↔SV
불일치가 valid SV 토큰을 예기치 못하게 만들 수 있음). 억제 불가.

### VITA-W2003 · `W-PARSE-IMPLICIT-NET` (Warning)
**`default_nettype wire` 하에서 암시적 net 추론.** 미선언 식별자를 net 문맥에 써서 1-bit net으로
암시 선언될 때(IEEE 1364 §19.2 / 1800 §22.8). `default_nettype none`이면 같은 코드가 hard error.
오타(`enabel`)가 silently 새 wire가 되는 고전 버그라 경고. 유효 `default_nettype`은 sticky·파일 간
상속(RULE S)이라 이 경고 유무는 컴파일 순서에 의존.
```
assign y = a & enabel;            // 'enabel' 오타 -> 암시 1-bit wire
warning[VITA-W2003]: implicit net 'enabel' inferred (default_nettype wire)  --> m.sv:3:18
```
**해결:** net 명시 선언 또는 오타 수정. 프로덕션 RTL은 `` `default_nettype none `` 권장.
`-Wno-W-PARSE-IMPLICIT-NET`(또는 인라인 `lint_off`)로 억제, `-Werror=`로 승격.

> **현 구현 정책(P2-12 명문화):** vitamin v1은 **implicit net을 생성하지 않는다** — 미선언
> 식별자 참조는 일률적으로 `E-ELAB-UNRESOLVED-NAME`(E3010) hard error다. 즉 현재 동작은
> IEEE 기본(`default_nettype wire`)이 아니라 사실상 `` `default_nettype none ``이며, 이 코드는
> implicit-net 추론을 구현하는 시점까지 **예약(emitter 0)** 상태다. 오타가 조용히 wire가 되는
> 사고 클래스가 원천 차단되는 보수적 선택.

---

## 3xxx · ELABORATE

### VITA-E3001 · `E-ELAB-MULTIDRIVER` (Error)
**한 net이 복수 구조적 드라이버로 충돌 구동.** 평탄화 sim-ir에서 한 net(또는 비트범위)이 둘
이상의 구조적 드라이버(복수 `assign`, output 포트, gate)로 구동되고 `--multi-driver` 정책이
`error`(기본, bucket B 입력)일 때. 값은 IEEE §6.6 4-state wired-logic으로 항상 *해소*되며, 이
코드는 그 충돌을 hard fail로 볼지를 게이트한다. `%m` 계층 경로와 함께 보고.
```
wire w;  assign w = a;  assign w = b;     // w에 두 번째 구조적 드라이버
$ velab -s top --multi-driver error   ->  VITA-E3001 at top.w
```
**해결:** 단일 드라이버로 줄이거나 의도된 wired-logic을 명시(`tri`/open-drain). 복수 드라이버가
의도면 `velab --multi-driver warn`으로 정책을 낮춘다 — 같은 `E-ELAB-MULTIDRIVER` 코드가 Warning
severity로 바뀌고 elaboration이 계속된다(`--multi-driver`는 bucket B 해시 입력이라 스냅샷 재생성).

### VITA-E3002 · `E-ELAB-PORT-MISMATCH` (Error)
**인스턴스 포트 연결이 모듈 포트 선언과 비호환.** 모듈에 없는 named 포트 `.foo()`, 포트 수를
넘는 positional 연결, 방향/종류 비호환 등 바인딩 자체가 무의미할 때(IEEE 1800 §23.3.2). 폭
불일치(복구 가능 `W-ELAB-WIDTH-TRUNC`)와 구별 — 인스턴스를 형성할 수 없다. `%m` + AST span 첨부.
```
module child(input a, output y); endmodule
child u0(.a(1'b0), .z(y));         // .z 는 child 의 포트가 아님
->  VITA-E3002 at top.u0 (no port `z` on module `child`)
```
**해결:** 선언된 포트로 연결 수정(이름/positional 수·방향). 미연결 포트는 `.z()`로 비운다.
억제 불가.

### VITA-E3003 · `E-ELAB-UNRESOLVED-INSTANCE` (Error)
**인스턴스화한 모듈을 컴파일된 설계 단위로 해소 불가.** 계층 평탄화 중 인스턴스 타깃 모듈이 work
라이브러리·`-L` compose·`-y`/`-v` 검색 어디에도 없을 때. vcmp는 단위를 고립 컴파일하므로(parse가
마지막 언어 의존 단계) 누락 참조는 elaborate 시점에야 드러난다.
```
alu u_alu(.a(x), .b(y));          // 모듈 alu 가 work 에 컴파일된 적 없음
$ vcmp top.sv && velab -s top   ->  VITA-E3003: cannot resolve instance `u_alu` of module `alu`
```
**해결:** 누락 단위 소스를 vcmp filelist에 추가, 또는 `-L <lib>`/`-y <libdir>`로 발견 가능하게,
또는 모듈명 오타 수정. (`-L`/`-y` 해소 내용은 bucket B 해시 입력.) 억제 불가.

### VITA-E3004 · `E-ELAB-USER-ERROR` (Error)
**elaboration 시점 `$error` 발화.** 절차 블록 밖(모듈 레벨·generate)에서 elaboration 평가된
`$error`가 design-time 검사 실패로 발화(IEEE §20.11). `elaborate` 크레이트가 runtime과 같은
LogEvent 경로로 내되 **sim_time 없음**, span은 AST 직접. IEEE상 기록 후 **계속**(중단 아님).
```
if (DEPTH <= 0) $error("DEPTH=%0d must be positive", DEPTH);   // elaboration-time
$ velab -s fifo   ->  VITA-E3004: DEPTH=0 must be positive (elaboration continues)
```
**해결:** 파라미터/오버라이드를 고친다(`-G DEPTH=16`). `$error`만으로는 중단 안 함(중단 원하면
RTL에서 `$fatal`). `-Wno-E-ELAB-USER-ERROR`로 억제.

### VITA-F3005 · `F-ELAB-USER-FATAL` (Fatal)
**elaboration 시점 `$fatal` 발화.** 모듈 레벨·generate에서 평가된 `$fatal(n,…)`이 design-time
검사 실패로 발화(IEEE §20.11). elaboration을 **즉시 중단**하고 스냅샷을 만들지 않는다("no simv"
analogue). `n`은 exit-stats verbosity만 제어. velab/vita는 **exit class 1**(staleness class 2와
구별).
```
if (IN_W < 1 || IN_W > 64) $fatal(1, "IN_W=%0d out of range", IN_W);
$ velab -s mac -G IN_W=128   ->  VITA-F3005, elaboration aborts, exit 1, no snapshot
```
**해결:** 유효 파라미터 공급(`-G IN_W=32`) 또는 guard 수정. Fatal은 억제 불가(중단을 un-abort
못함). class-1 + 이 코드 = RTL/파라미터 실패(재빌드 필요한 staleness 아님).

### VITA-I3006 · `I-ELAB-USER-INFO` (Info)
**elaboration 시점 `$info` 발화.** 모듈 레벨·generate에서 평가된 `$info`가 정보 출력(해소된
파라미터·선택된 generate 구성 보고). Info severity, sim_time 없음, exit 무관.
```
$info("elaborating dcache with WAYS=%0d", WAYS);
->  info[VITA-I3006]: elaborating dcache with WAYS=4
```
**해결:** 조치 불요. `-q`/`-Wno-I-ELAB-USER-INFO`로 조용히.

### VITA-W3007 · `W-ELAB-USER-WARNING` (Warning)
**elaboration 시점 `$warning` 발화.** 모듈 레벨·generate에서 평가된 `$warning`이 합법이지만
의심스러운 design-time 조건을 표시(IEEE §20.11). 계속 진행, `-Werror` 승격 시에만 nonzero.
```
if (LATENCY < 1) $warning("LATENCY=%0d is unusually small", LATENCY);
$ velab -s pipe -G LATENCY=0   ->  warning[VITA-W3007]: LATENCY=0 is unusually small
```
**해결:** 파라미터 조정 또는 의도면 수용. `-Wno-`로 억제, `-Werror=W-ELAB-USER-WARNING`로 RTL
수정 없이 CI 실패(통합 게이트).

### VITA-W3008 · `W-ELAB-WIDTH-TRUNC` (Warning)
**폭 불일치가 묵시적 truncation/extension으로 해소.** 포트 연결·`assign`·파라미터 식에서 소스
폭이 타깃보다 크거나(상위 비트 손실) 작을 때. IEEE §11.6.1상 묵시 size cast는 합법이라 진행하되,
silent truncation은 고전 오류 원인이라 표면화. 두 폭 + `%m` + span 보고.
```
wire [7:0] wide = 8'hAB;  wire [3:0] narrow;
assign narrow = wide;             // 8 -> 4 비트, 상위 nibble 손실
->  warning[VITA-W3008]: width 8 truncated to 4 (top, assign narrow)
```
**해결:** cast를 명시(`wide[3:0]`)하거나 폭을 맞춘다. `-Wno-W-ELAB-WIDTH-TRUNC`(또는 인라인
`lint_off`, RULE S로 소스 해시 반영)로 억제, `-Werror=`로 승격.

### VITA-W3011 · `W-ELAB-CASEZ-APPROX` (Warning)

`casez` 라벨에 **명시적 `x`** 비트가 있다. v1은 `reduction_or(scrut^label)!==1` 트릭으로 x/z를 모두 don't-care로 마스킹하므로, 엄밀한 `casez`(z/`?`만 와일드카드)와 달리 이 explicit-x를 don't-care로 취급한다(§08 알려진 단순화). `?`/`z` 라벨에는 발생하지 않으며 `casex`는 정의상 정확하다.

```verilog
casez (sel)
  4'b10x0: r = 1;   // W-ELAB-CASEZ-APPROX — explicit x는 z처럼 don't-care 처리됨
  default: r = 0;
endcase
```

**해결:** 의도가 don't-care면 `?`/`z`로 표기(무경고). 엄밀한 x-비교가 필요하면 Phase-1.x의 정밀 casez를 기다린다.

### VITA-E3009 · `E-ELAB-UNSUPPORTED` (Error)
**elaborate가 아직 지원하지 않는 구문.** v1 슬라이스는 단일 top module의 net/var 선언 +
continuous `assign`만 lowering한다. 절차 블록(`always`/`initial`), 모듈 인스턴스, generate,
function/task, user `Call`, real/string 리터럴 등은 미구현이라 이 코드로 표면화하고 elaboration을
중단한다. (각 구문은 후속 슬라이스에서 구현 예정.)
```
module m; always @(*) y = a;  endmodule
->  error[VITA-E3009]: procedural blocks are not yet supported (v1)
```
**해결:** 해당 구문을 제거하거나, 지원되는 후속 버전을 기다린다.

### VITA-E3010 · `E-ELAB-UNRESOLVED-NAME` (Error)
**선언되지 않은 net/variable 참조.** `assign`/식/lvalue에서 심볼 테이블에 없는 이름을 참조할 때.
모듈 해소용 `E-ELAB-UNRESOLVED-INSTANCE`(E3003)와 구분되는, net-name 전용 코드다.
```
module m; wire y; assign y = z;  endmodule   // z 미선언
->  error[VITA-E3010]: undeclared net/variable `z`
```
**해결:** 누락된 net/reg 선언을 추가하거나 오타를 바로잡는다.

### VITA-E3018 · `E-ELAB-LVALUE-KIND` (Error)
**대입 종류와 대상 kind 불일치 — continuous→variable / procedural→net.** 사용자
`assign`이 `reg`/`integer`/`real` 변수를 구동하거나, `initial`/`always` 절차 대입이
`wire`를 대상으로 할 때(iverilog도 양방향 모두 거부; Verilator CONTASSREG/PROCASSWIRE).
SV `logic`은 IEEE 1800상 단일 continuous 드라이버 또는 절차 구동 어느 쪽도 적법하므로
양쪽 모두 통과한다. 포트 바인딩이 합성한 암묵 연결은 검사 대상이 아니다(IEEE 1800
§23.3.3 var-port 적법).
```
module m; reg r; assign r = 1'b1; endmodule
->  error[VITA-E3018] E-ELAB-LVALUE-KIND: continuous assign drives variable `r` (reg)
```
**해결:** 대상 선언을 `wire`↔`reg`(또는 SV `logic`)로 맞추거나 대입 형태를 바꾼다.
(부록 A에서 본문 승격.)

### VITA-W3056 · `W-ELAB-FEATURE-LIMIT` (Warning)
**적법 구문을 수용하되 단순화 — elaborate 범용 simplification 경고.** v1이 의도적으로
근사/생략하는 적법 구문 전반(미연결 포트, inout 단방향 근사, intra-assignment delay 드롭,
미지원 `$task` skip, fork-local decl 공유 스코프 등)에 부여되는 catch-all 코드.
(부록 A에서 본문 승격 — Verilator MINTYPMAXDLY/RISEFALLDLY류 대응. 종전에는 이 부류가
전부 `W-ELAB-WIDTH-TRUNC`로 잘못 찍혀 코드 라우팅이 깨져 있었다; W3008은 실제 폭-절단
경고 구현 전까지 본문 예약 상태로 남는다.)
```
module m(input i); endmodule  module t; m u1(); endmodule   // 포트 미연결
->  warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: input port `i` left unconnected
```
**해결:** 대부분 무해(의도 확인용). `-Wno-`/`-Werror=`로 억제·승격.

---

## 4xxx · RUNTIME

### VITA-E4001 · `E-RUN-ASSERT-FAIL` (Error)
**action block 없는 assert 실패 — 묵시 `$error` severity.** immediate `assert(...)` 또는
concurrent `assert property(...)`가 거짓이고 `else`(fail) 블록이 없을 때, IEEE §16.3상 묵시
`$error` severity를 갖는다. RTL `$error`와 **같은 게이트**로 라우팅, 실패 플래그 기록 후 계속.
(immediate `assert`(✅ else $error/$fatal severity)·deferred `assert #0`/`assert final`(✅ 22탄)·
concurrent `assert property`(SVA, ✅ Phase-3 — full 시퀀스 서브셋·multi-clock·named property/sequence까지)
모두 구현됨 — 단, 실패는 합성 clocked-always 체커 또는 severity 팩터의 `$error`
(→`E-RUN-USER-ERROR` E4003)로 방출되어 이 코드 자체는 **예약-미발화**.)
```
always @(posedge clk) assert (state != ERR_STATE);   // state==ERR_STATE 시 실패
->  error[VITA-E4001] E-RUN-ASSERT-FAIL: assertion failed at tb.dut.fsm (state.sv:42) time=1750ns
```
**해결:** 설계/벤치를 고치거나 명시 action block(`assert(...) else $warning(...)`). `-Wno-`/
`-Werror=`로 억제·승격. corpus(`--error-exit` 기본 ON)에선 1회 발화가 FAIL — `expect_codes`로
코드 assert.

### VITA-E4002 · `E-RUN-RANGE` (Error)
**런타임 배열 인덱스/비트·파트 셀렉트 범위 초과.** 동적 인덱스가 선언 범위를 벗어날 때. IEEE
§11.5.1상 범위 밖 셀렉트는 `x`를 읽고(쓰기는 무시) crash하지 않으므로, 표준 read-x/ignore-write
값 의미를 지키며 동시에 이 진단으로 silent corruption을 보이게 한다. sim-ir는 span-free라
**§7 side-table**로 file:line 복원 + sim_time.
```
logic [7:0] mem [0:15];  int idx = 20;  $display("%0h", mem[idx]);  // 20 > 15
->  error[VITA-E4002] E-RUN-RANGE: index 20 out of range [0:15] at tb.mem (mem.sv:9)
```
**해결:** 셀렉트 전에 인덱스 검증/clamp 또는 배열 크기 확대. 값 의미는 표준(read x), 기록 후 계속.
`-Wno-E-RUN-RANGE`로 억제, `-Werror=`로 CI 중단. `(source location unavailable)`이면 위치 없는
스냅샷 — `W-RUN-NO-LOCATIONS` 참조.

### VITA-E4003 · `E-RUN-USER-ERROR` (Error)
**시뮬레이션 시점 RTL `$error` 발화.** 절차/시뮬 문맥(initial/always/task, 또는 SVA `else
$error`)에서 실행. IEEE §20.11상 메시지 출력 후 **계속**(`$finish` 안 부름, exit 클래스 자체로는
무변). hdl-builtins가 LogEvent로 방출 → `$display`와 sim-time 순 인터리브. severity+file:line(§7)+
`%m`+sim_time 조립.
```
if (dut_result !== expected) $error("MISMATCH got=%0h exp=%0h", dut_result, expected);
->  error[VITA-E4003] E-RUN-USER-ERROR: MISMATCH got=ab exp=cd at tb (tb.sv:31) time=1200ns
```
**해결:** 의도된 벤치 출력 — 벤치가 표시한 조건을 고친다. 기본은 IEEE(계속, exit 무관);
`--error-exit`로 `$error` 발화 시 nonzero(corpus가 이걸로 게이트). `-Wno-`/`-Werror=`(통합 게이트).

### VITA-F4004 · `F-RUN-FATAL` (Fatal)
**시뮬레이션 시점 RTL `$fatal` 발화 — 묵시 `$finish`로 중단.** IEEE §20.11상 `$fatal`은 묵시
`$finish`로 즉시 종료. Fatal 진단 후 시뮬 단계 중단, **exit class 1**(staleness class 2와 구별).
앞 `n`(0/1/2)은 shell code가 아니라 exit-stats verbosity(0=silent,1=time+loc,2=+stats).
```
if (cfg_invalid) $fatal(1, "bad config word %0h", cfg);
->  fatal[VITA-F4004] F-RUN-FATAL: bad config word deadbeef at tb.dut (dut.sv:88) time=500ns (exit 1)
```
**해결:** 치명 조건 해소 — `$fatal`은 작성자의 명시적 "계속 불가". Fatal은 continue로 억제 불가.
corpus는 class-1 내 compile-fail vs runtime-fail을 요약 MsgCode로 구분(`F-RUN-FATAL`=돌다 중단).
(elaboration 시점 `$fatal`은 `F-ELAB-USER-FATAL`.)

### VITA-I4005 · `I-RUN-USER-INFO` (Info)
**시뮬레이션 시점 RTL `$info` 발화.** 절차/시뮬 문맥에서 정보 출력(예 "test PASSED"). IEEE
§20.11상 순수 정보, 계속, exit 무관. severity+file:line+`%m`+sim_time을 붙여 plain `$display`
(severity 없는 RtlOutput)과 구별.
```
if (pass) $info("test PASSED (%0d vectors)", n);
->  info[VITA-I4005] I-RUN-USER-INFO: test PASSED (256 vectors) at tb (tb.sv:54) time=9000ns
```
**해결:** 조치 불요(exit 무관). `-q`/`-Wno-I-RUN-USER-INFO`로 조용히(단 `-q`는 stdout 복사만,
자동 로그파일엔 기록됨).

### VITA-W4006 · `W-RUN-NO-LOCATIONS` (Warning)
**로드한 `.velab`의 위치 side-table이 제거됨 — 런타임 진단이 file:line 불가.** §7 위치
side-table은 선택적이며 release 스냅샷은 없이 배포될 수 있다(D4). 없으면 런타임 진단이 caret
대신 `(source location unavailable)`로 degrade — 시뮬레이터는 절대 crash 안 하고 code/severity/
sim_time/`%m`은 출력. 로드 시점 **1회** 경고.
```
$ vrun build/cpu.velab
warning[VITA-W4006] W-RUN-NO-LOCATIONS: snapshot has no location side-table;
  runtime diagnostics will omit file:line (rebuild without --strip-locations, or vrun --rebuild)
```
**해결:** 위치 포함으로 재-elaborate(side-table은 **기본 포함**이며 `velab --strip-locations`로만
빠진다 — strip을 안 하면 됨), 또는 `vrun --rebuild`. 권고성이라 시뮬은 정상 진행.
`-Wno-W-RUN-NO-LOCATIONS`로 억제(의도적 strip 스냅샷), `-Werror=`로 CI 실패.

### VITA-W4007 · `W-RUN-USER-WARNING` (Warning)
**시뮬레이션 시점 RTL `$warning` 발화.** IEEE §20.11상 경고 출력 후 계속, 도구 억제 가능.
compile-time warning과 **같은 게이트**를 지나므로 `-Werror=W-RUN-USER-WARNING`이 RTL `$warning`을
소스 수정 없이 CI 실패로(GHDL `--warn-error` 선례). 승격 시에만 nonzero.
```
if (fifo_almost_full) $warning("fifo near full: depth=%0d", depth);
->  warning[VITA-W4007] W-RUN-USER-WARNING: fifo near full: depth=14 at tb.dut.u_fifo time=620ns
```
**해결:** 경고 조건 처리 또는 수용. `-Wno-`/`--suppress=`로 억제, `-Werror=`로 승격.
always-logged spine이 아니라 `-q`는 stdout 복사만 영향(자동 로그엔 기록). `--error-limit`은
warning 미계수.

### VITA-F4016 · `F-RUN-NO-CONVERGE` (Fatal)
**델타 한도 내 수렴 실패 — zero-delay 루프/조합 발진.** 한 타임스텝의 델타 사이클 수가
`max_deltas`(기본 1,000,000)를 초과: cont-assign 발진(`assign a = ~a`), 타이밍 제어 없는
절차 루프(`always begin a=~a; end`), 또는 0-딜레이 피드백. 시뮬 즉시 중단, exit class 1.
(부록 A에서 본문 승격 — Verilator DIDNOTCONVERGE 대응.)
```
assign a = ~a;
->  fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000) exceeded at time 0
```
**해결:** 피드백 경로를 끊거나(레지스터 삽입) 루프에 타이밍 제어(`#`/`@`)를 넣는다.
의도된 깊은 체인이면 `SimOpts.max_deltas` 상향. 억제 불가(Fatal).

### VITA-W4018 · `W-RUN-VCD-OPEN-FAIL` (Warning)
**`$dumpfile` 경로 열기 실패 — VCD 없이 시뮬 계속.** `$dumpvars` 시점에 dump 파일을
생성할 수 없을 때(없는 디렉터리, 권한, 읽기 전용 FS). 주 산출물이 조용히 증발하는 대신
경고를 남기고 시뮬은 정상 진행(파형만 없음). exit class 무변.
```
$dumpfile("/no/such/dir/wave.vcd"); $dumpvars;
->  warning[VITA-W4018] W-RUN-VCD-OPEN-FAIL: cannot open VCD dump file '/no/such/dir/wave.vcd': No such file or directory (os error 2)
```
**해결:** 디렉터리 생성/권한 수정 또는 `-o`로 출력 경로 재지정. `-Werror=`로 CI 승격 가능.

### VITA-W4019 · `W-RUN-VCD-WRITE-FAIL` (Warning)
**VCD flush/write 실패 — 파형이 잘렸을 수 있음.** 시뮬 종료 시 `finalize_vcd`의 flush가
실패할 때(디스크 풀, I/O 에러). 마지막 완료 write까지는 유효한 truncated VCD가 남는다.
```
->  warning[VITA-W4019] W-RUN-VCD-WRITE-FAIL: VCD flush failed: disk full
```
**해결:** 디스크 공간/마운트 상태 확인 후 재실행. `-Werror=`로 CI 승격 가능.

### VITA-W4020 · `W-RUN-DYN-DEGRADE` (Warning)
**dynamic-storage 연산이 명세된 degraded 경로를 탔음** (v5 (C)): `new[n]`의 n이 X/Z →
빈 배열, 범위 밖 인덱스 read → X / write → 무시, queue empty pop → X, 직접-대입
위치 밖의 pop(NBA rhs·중첩 식 등) → X(미-pop), 원소 cap(1<<24) 초과 push/new/assoc-write
→ drop/clamp, assoc X/Z 키 read → X / write·delete(k) → 무시 / exists → 0, assoc
미존재 키 read → X, concat-lvalue 안의 assoc 원소 chunk → 무시
등 (설계 문서 2026-06-10 §4, IEEE §7.8.6 invalid-index). 단 queue `q[size()] = v`는
**push_back 동등으로 합법-무음**(IEEE §7.10.1 — 경고 아님)이고, assoc **미존재 키
write는 원소 생성**(§7.8), **`delete(k)` 미존재 키는 무음 no-op**(§7.9) — 셋 다 경고
아님. **핸들 net당 1회만** 발행(warn-once 래치) — 루프 안의 degraded 연산이 진단
스트림을 폭주시키지 않는다.
```
->  warning[VITA-W4020] W-RUN-DYN-DEGRADE: new[] size is X/Z; array degraded to empty
```
**해결:** 크기/인덱스 식의 X 근원을 수정. `-Werror=`로 CI 승격 가능.

### VITA-W4021 · `W-RUN-DUMP-MULTI` (Warning)
**두 번째 이후의 `$dumpvars` 호출이 무시됨** (Phase-1.x ⑤b: 첫 호출이 VCD를 열고
필터(depth/scope/net 인자)를 확정 — VCD 헤더는 한 번만 쓸 수 있으므로 추가 호출의
인자 합집합(LRM 누적 모델)은 v1 컷). 추가 호출은 이 경고 1회(run당) 후 no-op.
```
->  warning[VITA-W4021] W-RUN-DUMP-MULTI: extra $dumpvars call ignored (v1: the first call wins)
```
**해결:** 모든 덤프 대상을 첫 `$dumpvars` 호출에 모으기.

---

## 8xxx · FILELIST

### VITA-W4022 · `W-RUN-BAD-FD` (Warning)

파일 연산($fdisplay/$fwrite/$fclose)이 유효하지 않거나 이미 닫힌 디스크립터를 받음 — 해당 연산은 무시된다(iverilog 동작 동일). fd당 1회만 경고.

```
->  warning[VITA-W4022] W-RUN-BAD-FD: file operation on an invalid/closed descriptor ignored
```

- **원인**: `$fclose` 후 같은 fd로 쓰기, X/Z fd, `$fopen` 실패(0) 값 사용.
- **해결**: fd 수명을 확인하고 `$fopen` 반환값이 0이 아닌지 검사.

### VITA-W4023 · `W-RUN-READMEM` (Warning)

`$readmemb`/`$readmemh`가 파일을 열지 못했거나, 주소 지시어 없는 파일의 토큰 수가 요청 범위와 안 맞음(부족/초과). 메모리는 부분 적재되고 실행은 계속된다(iverilog는 missing file을 "ERROR" 텍스트로 찍지만 exit 0 — vitamin은 exit 패리티를 유지하며 Warning으로 분류).

```
->  warning[VITA-W4023] W-RUN-READMEM: $readmemb/h problem (missing file / word-count mismatch); memory partially loaded
```

- **원인**: 경로 오타, 토큰 수 ≠ 범위 크기, 범위 밖 `@addr`.
- **해결**: 파일 경로/내용을 범위와 맞추거나 명시적 start/finish 인자를 사용.

### VITA-F4024 · `F-RUN-CLASS-LIMIT` (Fatal)

N7 class 객체 수가 `SimOpts::max_class_objs`(기본 1,000,000) 한도를 초과 — class 힙은 GC가 없어 절대 회수되지 않으므로, 루프 안의 무한 `new()`는 한도 없이 자란다. 한도 초과 시 묵시적 `$finish`(graceful, exit 1)로 OOM 대신 loud하게 중단한다(delta-limit `F-RUN-NO-CONVERGE`와 동일한 자원-한도 패턴).

```
->  fatal[VITA-F4024] F-RUN-CLASS-LIMIT: class object budget (1000000) exceeded — likely an unbounded `new()` ...
```

- **원인**: 매 사이클 `h = new();`처럼 핸들을 덮어쓰며 객체를 무한 생성(이전 객체는 회수 불가).
- **해결**: 라이브 객체 재사용, 또는 의도된 대량 할당이면 `SimOpts::max_class_objs`를 상향.

### VITA-W4025 · `W-RUN-WIDE-ARITH` (Warning)

multi-word 산술(`*`/`/`/`%`/`**`)의 피연산자 폭이 `WIDE_ARITH_CAP`(2^20, `MAX_NET_WIDTH`와 동일)을 초과 — 선언-합법 net은 2^20 이하지만 replication concat(`{16{a}}`)이 *피연산자*를 16M-bit까지 부풀릴 수 있다. 초과 시 super-linear 커널(`*` O(n²)·복원 `/`·`%` O(bits·n)·`**` square-multiply)이 수십~수백 초 stall하므로 결과를 X로 poison한다(div-by-zero degrade 선례). `simulate` 시작 시 해당 노드가 존재하면 1회 loud 경고. `+`/`-`는 O(n)이라 어떤 폭에서도 정확.

```
->  warning[VITA-W4025] W-RUN-WIDE-ARITH: multi-word arithmetic exceeds the width cap; result poisoned to X
```

- **원인**: replication/concat로 2^20-bit를 넘는 곱셈·나눗셈·거듭제곱(예: `{2{a}} * {2{a}}`, `a`는 2^20-bit).
- **해결**: 피연산자 폭을 `MAX_NET_WIDTH` 이하로 줄인다(2^20-bit 초과 산술은 v1 범위 밖).

### VITA-E8001 · `E-FLIST-CYCLE` (Error)
**filelist 사이클 — 활성 스택에 이미 있는 `.f`를 재포함.** 중첩 `-f`/`-F`가 (베이스 해소+lexical
canonical 후) 현재 열린 `.f` active-stack의 경로로 해소될 때. 평탄화는 트리여야 하므로 back-edge는
silent 스킵 금지 — 전체 체인을 보고. diamond(다른 가지서 도달, 이미 pop)는 사이클 아님.
```
# build.f: -f sub.f      # sub.f: -f build.f
error[VITA-E8001] E-FLIST-CYCLE: filelist cycle: build.f -> sub.f -> build.f
```
**해결:** 사이클을 끊는다(자기/조상 참조 제거, 공유 내용은 leaf `.f`로 분리 = diamond 합법).
억제 불가(exit class 3).

### VITA-E8002 · `E-FLIST-DEPTH` (Error)
**중첩 깊이가 backstop cap(256) 초과.** 중첩은 사실상 무제한(사이클 가드)이나, 비순환 폭주
체인이 OS 스택을 소진하기 전에 256 프레임에서 중단한다.
```
# 생성된 체인 f0.f -> f1.f -> ... (사이클 아님, 그냥 깊음)
error[VITA-E8002] E-FLIST-DEPTH: filelist nesting exceeded depth cap 256 at f256.f
```
**해결:** 생성을 평탄화(대개 생성기가 한두 단계로) 또는 독립 top-level 호출로 분리. 억제 불가
(exit class 3).

### VITA-E8003 · `E-FLIST-DUP-CTX-CONFLICT` (Error)
*(silent-dedup arm 구현 2026-06-11; **CONFLICT arm도 같은 날 구현** — 추적되는 sticky
컨텍스트는 상속 `` `timescale``(comment/string-aware 라이트 스캔, 중복 존재 시에만 컨텍스트
워크 실행). `default_nettype`은 v1 미지원(E3010 정책)이라 컨텍스트 밖; RULE S 매니페스트
해시 통합은 worklib 도입 시.)*
**같은 canonical 소스가 다른 상속 sticky 컨텍스트로 두 번.** 소스는 기본 dedup 안 함(중복 모듈은
`E-DUP-UNIT`). 같은 canonical 경로가 두 번인 경우만 dedup하되, 두 occurrence가 다른 상속 sticky
디렉티브(`timescale`/`default_nettype`, RULE S) 컨텍스트면 같은 입력이 아니므로 silent dedup이
한쪽을 떨군다 → hard error로 양쪽 컨텍스트 제시.
```
# a.f: `timescale 1ns/1ps  then shared.sv
# b.f: `timescale 1ps/1ps  then shared.sv   (같은 경로, 다른 상속 timescale)
error[VITA-E8003] E-FLIST-DUP-CTX-CONFLICT: rtl/shared.sv included twice under differing sticky context
```
**해결:** 두 occurrence를 일치시키거나(동일 sticky 디렉티브 선행, 또는 한 번만 포함), 파일이
자기 `timescale`/`default_nettype`를 self-contained하게. 억제 불가(silent dedup은 RULE S 매니페스트
해시를 오염; exit class 3).

### VITA-E8004 · `E-FLIST-GLOB` (Error)
**filelist의 glob/wildcard 거부.** 소스/디렉터리 토큰의 `*`/`?`/`[...]`는 거부 — readdir 순서가
플랫폼 불안정이라 RULE S 정렬을 비결정으로 만들고 §5 "3-OS 바이트 동일"을 깬다. silent 전개 안 함.
```
rtl/*.sv                          # 플랫폼마다 비결정
error[VITA-E8004] E-FLIST-GLOB: wildcard 'rtl/*.sv' not allowed; emit an explicitly sorted file list
```
**해결:** 명시적 정렬 경로로 대체. 생성 필요 시 생성기가 정렬해 명시 `.f`를 낸다. 억제 불가
(exit class 3).

### VITA-E8005 · `E-FLIST-NOT-FOUND` (Error)
**filelist 또는 참조 경로가 프레임 베이스 해소 후 없음.** `-f`/`-F` 타깃·소스·검색 디렉터리가
프레임 베이스(`-f`=invocation CWD, `-F`=`.f` 자기 디렉터리) 기준으로 존재하지 않을 때.
canonicalization이 case-fold를 안 하므로 대소문자만 다른 경로는 여기서 표면화(macOS
case-insensitive FS 충돌을 silent alias 대신 가시화).
```
-F ./ip/Core.f                    # 실제 파일은 ip/core.f (대소문자 차이)
error[VITA-E8005] E-FLIST-NOT-FOUND: cannot open './ip/Core.f' (base=file-dir)
```
**해결:** 경로(대소문자 포함)/베이스를 고친다. `-f`=CWD 상대, `-F`=파일 디렉터리 상대를 혼동하기
쉬움. `--dump-filelist`로 (origin, base, canonical-path) 확인. 억제 불가(exit class 3).

### VITA-E8006 · `E-FLIST-UNDEF-ENV` (Error)
**filelist의 미정의 환경변수 참조.** `$VAR`/`${VAR}`/`$(VAR)`가 환경에 없을 때. silent 빈
문자열 치환은 잘못된 경로(FS 루트로 붕괴 등) + 환경마다 다른 해시를 내어 재현성을 해치므로
hard-error.
```
$RTL_ROOT/cpu/alu.sv              # RTL_ROOT 미export
error[VITA-E8006] E-FLIST-UNDEF-ENV: undefined environment variable 'RTL_ROOT' in build.f:1
```
**해결:** 변수 export 또는 구체/상대 경로로 대체. CI는 필요한 변수를 명시 설정. 억제 불가
(exit class 3).

### VITA-E8007 · `E-FLIST-WRONG-STAGE` (Error)
**filelist 디렉티브가 호출 단계의 버킷에 안 맞음.** 각 단계 전개기는 전체 `.f` 문법을 파싱하되,
호출 단계가 소유하지 않는 버킷의 디렉티브는 silent no-op이 아니라 hard error. 예: `velab -f x.f`에
`+define+`(전처리/bucket A) — velab엔 전처리 패스가 없어 무시하면 의도 위반.
```
# elab.f: -s top  /  +define+WIDTH=8     # 전처리 디렉티브 — elaborate에 무효
error[VITA-E8007] E-FLIST-WRONG-STAGE: '+define+WIDTH=8' is a vcmp-stage flag, invalid during elaborate
```
**해결:** 소유 단계로 옮긴다(`+define+`/`+incdir+`/`-y`는 vcmp, `-s`/`-G`/`-L`은 velab), 또는
union을 받는 원샷 `vita` 사용. 억제 불가(exit class 3).

### VITA-W8008 · `W-FLIST-MIXED-BASE` (Warning)
**`-F` 프레임 안의 `-f` 줄이 재배치 가능 서브트리를 CWD에 re-anchor.** `-F`는 자기 디렉터리 기준
해소라 재배치 가능(벤더 IP)인데, 내부 `-f` 줄은 그 서브트리를 invocation CWD에 re-anchor해
재배치성을 깬다 — 거의 항상 벤더 패키징 버그. 의미는 유효해 경고.
```
# vendor.F: -F ./rtl/core.F  /  -f ./rtl/extra.f   # extra.f 를 CWD에 re-anchor
warning[VITA-W8008] W-FLIST-MIXED-BASE: -f inside -F frame re-anchors to CWD (relocatability lost)
```
**해결:** `-F` 트리 안에선 중첩 포함도 `-F` 사용. CWD anchor가 의도면 `-Wno-W-FLIST-MIXED-BASE`로
억제, `-Werror=`로 승격.

### VITA-W8009 · `W-FLIST-OVERRIDE` (Warning)
**단일값 knob이 두 곳에서 지정 — last-wins override 적용(항상 로깅).** 단일값 elaborate knob
(`--top-module`/`-s`, `--std`, `--timescale`, `--multi-driver`)이 평탄 `-f`/`-F`+명령줄 스트림의
두 곳 이상에 있을 때. 명령줄 토큰이 전개 뒤에 append되어 명령줄이 `.f`를 override. silent override를
막기 위해 **always-logged spine**(`-q`로도 억제 안 됨)으로 두 값·출처·승자를 보인다.
```
# build.f: --top-module dut_b
$ velab -s dut_a -f build.f
warning[VITA-W8009] W-FLIST-OVERRIDE: --top-module 'dut_b' (build.f:1) overridden by 'dut_a' (command line)
```
**해결:** 의도된 override(`velab -s top2 -f build.f`)는 지원 워크플로 — 경고는 정보성, 진행됨.
노이즈를 없애려면 knob을 한 곳에만(빌드 의도는 명령줄 권장). `-Werror=W-FLIST-OVERRIDE`로 strict
CI에서 실패.

---

## 9xxx · ARTIFACT / STALENESS

### VITA-E9001 · `E-ART-FORMAT-MISMATCH` (Error)
**산출물 magic 또는 format_version 불일치.** 헤더 전용 디코드(본문 역직렬화 전)에서 magic
(`VITWORKU`/`VELAB\0`) 또는 `format_version`이 이 빌드 기대와 다를 때 — foreign/손상 파일 또는
비호환 컨테이너 레이아웃. 본문을 안 읽으므로 misparse 불가, 재빌드 힌트와 함께 거부. (타입 형상의
`E-ART-SCHEMA-MISMATCH`보다 하위 게이트.)
```
error[VITA-E9001] E-ART-FORMAT-MISMATCH: top.velab has format_version=2, this vitamin expects 1
  hint: regenerate with `velab` (or `vcmp --clean`)
```
**해결:** 현재 도구로 재생성(`vcmp`/`velab`, 또는 `vcmp --clean`). 산출물은 항상 재생성 가능하므로
refuse-and-rebuild(version-GATE), silent 마이그레이션 없음. exit class 2. 억제 불가.

### VITA-E9002 · `E-ART-SCHEMA-MISMATCH` (Error)
**산출물 schema_hash가 도구의 구조적 타입-형상 해시와 다름.** 헤더의 `schema_hash`(D2/§5의
`#[derive(SchemaHash)]` 구조적 다이제스트)가 실행 도구에 컴파일된 값과 다를 때. 필드/variant
추가·삭제·재정렬·타입변경 — 또는 wire 영향 serde 속성(`rename`/`skip`/…) — 이 해시를 뒤집어,
비호환 형상 빌드의 산출물이 silent misparse되는 것을 헤더 단계에서 거부.
```
error[VITA-E9002] E-ART-SCHEMA-MISMATCH: top.velab schema 9f3c.., current tool schema 7a10..
  hint: rerun `velab`; the sim-ir type shape changed between builds
```
**해결:** 현재 도구로 재빌드(`velab`, 또는 `vcmp` 후 `velab`). version-GATE(refuse-and-rebuild),
마이그레이션 기계 없음. exit class 2. 억제 불가.

### VITA-E9003 · `E-ART-STALE-UPSTREAM` (Error)
*(검증 시임 구현 2026-06-11: `vrun --upstream <file.vu>`가 라이브 .vu를 재해시해 `.velab`의
`composite_input_hash`와 대조 — 불일치 = 이 에러, exit class 2. **2026-06-12 worklib v1로 자동
발견 체인 가동:** lib-모드 `.velab`은 소비한 매니페스트/blob/소스·include 다이제스트를 기록하고
bare `vrun`이 전부 라이브 재해시 — 어느 하나라도 다르면 이 에러.)*
**vrun이 라이브 소스 재해시 후 stale 스냅샷 거부(RULE V).** 매 실행 vrun이 상류 체인 전체를
라이브 소스에 재검증 — 소비 (lib:unit, src_sha256) 트리플마다 라이브 파일을 재전처리(상속 적용)해
다이제스트 재계산, 매니페스트 내용 해시·두 schema 해시 재확인. 라이브 해시가 스냅샷에 박힌 값과
다르면 stale이므로 틀린 결과를 내느니 거부. **mtime 안 씀** — 내용 해시만 건전.
```
$ velab -s top && vrun top.velab           # ok
$ echo '// edit' >> rtl/alu.sv ; vrun top.velab
error[VITA-E9003] E-ART-STALE-UPSTREAM: rtl/alu.sv digest changed since snapshot
  hint: rerun vcmp/velab, or vrun --rebuild
```
**해결:** stale 단계 재실행(`vcmp`/`velab`) 후 `vrun`, 또는 `vrun --rebuild`. exit class 2(RTL
버그 아닌 재빌드임을 CI가 앎; silent 재사용 없음). 억제 불가.

### VITA-E9004 · `E-ART-VERSION-GATE` (Error)
**생산 도구의 semver-major가 소비 도구와 비호환.** provenance/도구 지문에 기록된 생산 도구
semver-major가 비호환일 때(컨테이너 포맷·schema 해시 일치 여부와 무관). §5상 format_version/
schema_hash/tool-semver-major 불일치는 hard error + 재빌드 힌트, silent 재사용 없음. 빌드 지문
(git sha/dirty/profile)은 provenance 전용·staleness 키 아님(dirty 트리만으론 안 걸림).
```
error[VITA-E9004] E-ART-VERSION-GATE: top.velab produced by vitamin 2.x, this tool is 1.x
  hint: regenerate with this tool's `velab`, or install a matching vitamin
```
**해결:** 소비 도구와 major가 맞는 도구로 재생성하거나 맞는 버전 설치. refuse-and-rebuild;
마이그레이션은 산출물이 배포 포맷이 될 때까지 연기. exit class 2. 억제 불가.

### VITA-E9005 · `E-WORK-MANIFEST` (Error)
**work 라이브러리 매니페스트(`lib.toml`)가 없거나 정규형이 아님.** `-L`이 가리킨 디렉터리에
`lib.toml`이 없거나, 기계-작성 정규형(canonical v1)을 벗어나 strict 파서가 거부했거나, 논리
이름이 요청과 다를 때. 매니페스트는 vcmp `--work`가 기계-작성하며 수동 편집은 내용 해시를
바꿔 하류 스냅샷을 stale로 만든다(그건 E9003) — 이 에러는 그 이전, 읽기/파싱 자체의 실패다.
```
$ velab -L work=./w --top cpu
error[VITA-E9005] E-WORK-MANIFEST: ./w/lib.toml: not a canonical work manifest (line 1)
  hint: regenerate the library with `vcmp --work`, or fix the -L path
```
**해결:** `vcmp --work`로 라이브러리 재생성, 또는 `-L` 경로 교정. exit class 2(아티팩트 클래스 —
RTL 버그 아님). 억제 불가.

---

## 부록 A · 조사 기반 에러/경고 케이스 인벤토리 (공식 출처)

> 본문(§0~9)의 55개(현재 full-entry / `MsgCode` enum 등재 수, 2026-06-12 v7 기준)는 **MVP 설계·구현 대상** 코드다. 본 부록은 실제 시뮬레이터
> (Verilator · Icarus iverilog · VCS · Xcelium · GHDL) 공식 문서 + IEEE 1800/1364가 정의하는
> **추가 오류/경고 조건 107개를 미리 수집한 인벤토리**다 — 추후 구현 시 어떤 케이스를 처리해야
> 하는지 미리 드러내 구현을 용이하게 하려는 목적이다. **이 코드들은 아직 미구현(예약)** 이며,
> 구현될 때 본문 형식의 전체 항목(원인·예시·해결)으로 승격된다.
>
> **"예약"의 의미 (혼동 방지).** `MVP-SIM` 태그 코드는 **설계상 반드시 구현될 동작**이다 — '예약'은
> *본문 전체 항목(prose)·`MsgCode` enum 등재가 아직 없음*을 뜻하지, 그 **동작**이 선택적이라는 뜻이
> 아니다(이 점이 선택적 `LINT`/미래 `SVA`·`SV-TYPE`·`VHDL` 밴드와 다르다). 그래서 다른 문서가
> `W-PP-TIMESCALE-DEFAULT`(W1017)·`E-PP-TIMESCALE-PARTIAL`(E1011)·`W-PARSE-TIMESCALE-PARTIAL`(W2016)을
> "잠금"으로 참조하는 것은 그 *동작 규칙*이 확정됐다는 뜻이고, **코드 자체는 구현 시 본문+enum으로
> 승격**된다. bijection 게이트는 승격(=enum 등재) 후에만 그 코드를 대상으로 한다.

**scope 태그:** `MVP-SIM` = Verilog-2005/SV-subset 시뮬레이터가 반드시 다뤄야 함(구현 대상) ·
`LINT` = 스타일/린트(선택; Verilator 기본 off 다수) · `SVA`/`SV-TYPE`/`VHDL` = 예약 밴드(향후 기능).
*sev* 약어 Erro=Error.

**번호 부여 (거버넌스 보강):** 초기 36개(시드)는 알파벳순으로 부여했고(현재 본문 55개), **이후 추가는 해당 밴드의
다음 빈 번호를 영구 부여한다(재정렬·renumber 금지)** — 그래서 본 부록 번호는 알파벳순이 아니다.
mnemonic이 1차 안정 키임은 동일. 구현 전이므로 일부 번호는 향후 통합·재배치될 수 있다(미구현
인벤토리 한정).

**기존 코드로 흡수/중복 정리:** Verilator 세분 코드 일부는 기존 코드의 하위 케이스로 흡수 —
포트 폭 불일치(Verilator WIDTHCONNECT)는 `W-ELAB-WIDTH-TRUNC`(W3008)에, 다중 클럭 구동(CDC)
린트(Verilator MULTIDRIVEN)는 `E-ELAB-MULTIDRIVER`(E3001)에 교차참조. `` `timescale `` 부분
지정은 strict `E-PP-TIMESCALE-PARTIAL`(E1011)와 lenient `W-PARSE-TIMESCALE-PARTIAL`(W2016) 두
정책 코드로 제공한다(한 조건, 정책 선택). **out-of-box 기본은 lenient(W2016)** — iverilog `-Wtimescale`
관행. 선택은 `--timescale-policy <strict|lenient>`(§14 vcmp/velab 플래그, 버킷 A)로 한다. 아무 모듈도
지정 안 한 *전무* 사례는 별개 코드 `W-PP-TIMESCALE-DEFAULT`(W1017, 기저 `1ns/1ns`)가 담당한다(08).

### 1xxx · PREPROCESS  (9)

> 참고: E1004/E1005/E1013/W1007/W1008는 preprocessor MVP에서 본문 §1xxx로 승격되었다(2026-06-04).

| 번호 | mnemonic | sev | scope | 조건 | 출처 / 매핑 |
|---|---|---|---|---|---|
| E1006 | `E-PP-REDEFINE-DIRECTIVE` | Erro | MVP-SIM | Redefining a reserved compiler directive as a macro | IEEE 1364-2005 §19.3.1 |
| E1009 | `E-PP-UNDEF-MACRO-USE` | Erro | MVP-SIM | Use of an undefined text macro | IEEE 1364-2005 §19.3.1 |
| E1010 | `E-PP-UNBALANCED-CONDITIONAL` | Erro | MVP-SIM | Unbalanced `` `ifdef/`else/`endif `` | IEEE 1364-2005 §19.4 |
| E1011 | `E-PP-TIMESCALE-PARTIAL` | Erro | MVP-SIM | Some modules have `` `timescale `` and others do not (strict) | IEEE 1364-2005 §19.8 |
| E1012 | `E-PP-RESETALL-IN-MODULE` | Erro | MVP-SIM | `` `resetall `` inside a module/UDP declaration | IEEE 1364-2005 §19.6 |
| W1014 | `W-PP-IFDEF-VALUE-ZERO` | Warn | MVP-SIM | `` `ifdef `` tests a macro defined as 0 (definedness ≠ value) | Verilator PREPROCZERO |
| W1015 | `W-PP-BACKSLASH-SPACE` | Warn | MVP-SIM | Backslash followed by whitespace before newline | Verilator BSSPACE |
| W1016 | `W-PP-DEF-OVERRIDE` | Warn | MVP-SIM | Command-line `+define` overrides an in-source `` `define `` | Verilator DEFOVERRIDE |
| W1017 | `W-PP-TIMESCALE-DEFAULT` | Warn | MVP-SIM | No `` `timescale ``/`timeunit` anywhere and no `--timescale` → 기저값 `1ns/1ns` 적용 | iverilog -Wtimescale; IEEE 1364-2005 §19.8 (08 §no-timescale base) |

### 2xxx · PARSE  (17)

| 번호 | mnemonic | sev | scope | 조건 | 출처 / 매핑 |
|---|---|---|---|---|---|
| E2004 | `E-PARSE-UNSIZED-CONCAT` | Erro | MVP-SIM | Unsized operand inside concatenation/replication | Verilator WIDTHCONCAT; IEEE 1364-2005 §5.1.14 |
| E2005 | `E-PARSE-ZERO-REPL` | Erro | MVP-SIM | Zero replication count outside an enclosing concat | Verilator ZEROREPL; IEEE 1800 §11.4.12.1 |
| E2006 | `E-PARSE-RESERVED-KEYWORD` | Erro | MVP-SIM | Reserved keyword used as an identifier | IEEE 1364-2005 §3.7.2 / Annex B |
| E2007 | `E-PARSE-ILLEGAL-NUMBER` | Erro | MVP-SIM | Malformed number literal | IEEE 1364-2005 §3.5.1 |
| E2008 | `E-PARSE-UNTERMINATED-TOKEN` | Erro | MVP-SIM | EOF inside block comment or string literal | IEEE 1364-2005 §3.3/§3.6 |
| E2009 | `E-PARSE-END-LABEL` | Erro | MVP-SIM | Mismatched end/endmodule block label | Verilator ENDLABEL; IEEE 1800 §9.3.4 |
| E2010 | `E-PARSE-NOT-REDOP` | Erro | MVP-SIM | Logical-NOT before an unparenthesized reduction op | Verilator NOTREDOP |
| E2011 | `E-PARSE-NULL-PORTLIST` | Erro | MVP-SIM | Empty/null element in module port list | Xcelium *E,NULLLP; IEEE 1364-2005 §12.3 |
| E2012 | `E-PARSE-DECL-AFTER-STMT` | Erro | MVP-SIM | Declaration after a statement (Verilog-2005) | Xcelium *E,BADDCL; iverilog |
| W2013 | `W-PARSE-IMPLICIT-DIMENSIONS` | Warn | MVP-SIM | Port/net redeclaration missing dimensions | iverilog -Wimplicit-dimensions |
| W2014 | `W-PARSE-ANACHRONISM` | Warn | MVP-SIM | Deprecated/removed feature for the selected standard | iverilog -Wanachronisms |
| W2015 | `W-PARSE-NEWER-STD` | Warn | MVP-SIM | Construct requires a newer language standard | Verilator NEWERSTD; iverilog -g<year> |
| W2016 | `W-PARSE-TIMESCALE-PARTIAL` | Warn | MVP-SIM | Some modules set `` `timescale ``, others inherit (lenient) | Verilator TIMESCALEMOD; iverilog -Wtimescale |
| W2017 | `W-PARSE-DECL-AFTER-USE` | Warn | MVP-SIM | Identifier declared after first use (tolerated) | iverilog -Wdeclaration-after-use |
| W2018 | `W-LINT-ASCENDING-RANGE` | Warn | LINT | Ascending `[0:N]` packed range instead of `[N:0]` | Verilator ASCRANGE/LITENDIAN |
| W2019 | `W-LINT-DECL-FILENAME` | Warn | LINT | Module name ≠ file basename | Verilator DECLFILENAME |
| W2020 | `W-LINT-MISINDENT` | Warn | LINT | Misleading indentation suggests wrong grouping | Verilator MISINDENT |

### 3xxx · ELABORATE  (44)

> 참고: `E3009`/`E3010`는 elaborate v1에서 **본문 §3xxx** 코드로 승격되었다
> (`E-ELAB-UNSUPPORTED`/`E-ELAB-UNRESOLVED-NAME`). 아래 예약 인벤토리의
> `E-ELAB-DUP-DECL`·`E-ELAB-IMPLICIT-NET-NONE`는 충돌을 피하려 `E3005`/`E3006`으로
> 재배정했고, 옛 예약 `E3023 = E-ELAB-UNSUPPORTED`는 본문 `E3009`로 대체되어 제거했다.

| 번호 | mnemonic | sev | scope | 조건 | 출처 / 매핑 |
|---|---|---|---|---|---|
| E3005 | `E-ELAB-DUP-DECL` | Erro | MVP-SIM | Name declared twice in the same scope | IEEE 1364-2005 §4.11/§12.3.3 |
| E3006 | `E-ELAB-IMPLICIT-NET-NONE` | Erro | MVP-SIM | Undeclared net under `` `default_nettype none `` | IEEE 1364-2005 §19.2 |
| E3011 | `E-ELAB-MIXED-PARAM-OVERRIDE` | Erro | MVP-SIM | Mixed ordered and named parameter overrides | IEEE 1364-2005 §12.2.1 |
| E3012 | `E-ELAB-OVERRIDE-LOCALPARAM` | Erro | MVP-SIM | Override targets a localparam | IEEE 1364-2005 §4.10.2 |
| E3013 | `E-ELAB-GENLOOP-NONTERMINATING` | Erro | MVP-SIM | Generate-for loop non-terminating / genvar reuse | IEEE 1364-2005 §12.4 |
| E3014 | `E-ELAB-GENBLOCK-NAME-CONFLICT` | Erro | MVP-SIM | Generate-block name conflicts with another decl | IEEE 1364-2005 §12.4 |
| E3015 | `E-ELAB-UWIRE-MULTIDRIVER` | Erro | MVP-SIM | `uwire` net driven by more than one source | IEEE 1364-2005 §4.6.5; IEEE 1800 §6.6 |
| E3016 | `E-ELAB-HIER-NAME-UNRESOLVED` | Erro | MVP-SIM | Hierarchical name resolves to no object | IEEE 1364-2005 §12.4/§3.13 |
| E3017 | `E-ELAB-ASSIGN-INPUT` | Erro | MVP-SIM | Assignment to a module input port | Verilator ASSIGNIN; IEEE 1800 §23.3.3 |
| E3019 | `E-ELAB-CONTASS-INIT` | Erro | MVP-SIM | Variable both initialized and continuously assigned | Verilator CONTASSINIT |
| E3020 | `E-ELAB-PARAM-NO-DEFAULT` | Erro | MVP-SIM | Parameter without a required default | Verilator PARAMNODEFAULT |
| E3021 | `E-ELAB-FUNC-TIMING` | Erro | MVP-SIM | Time control or task call inside a function | Verilator FUNCTIMECTL; IEEE 1800 §13.4 |
| E3022 | `E-ELAB-PROTOTYPE-MISMATCH` | Erro | MVP-SIM | Out-of-block method def disagrees with prototype | Verilator PROTOTYPEMIS |
| E3057 | `E-ELAB-UNDEF-SYSTASK` | Erro | MVP-SIM | Call to an unrecognized system task/function | Xcelium *E,MSSYSTF; IEEE 1800 §20 |
| W3024 | `W-ELAB-WIDTH-EXPAND` | Warn | MVP-SIM | Rvalue narrower than lvalue, silently zero-extended | Verilator WIDTHEXPAND |
| W3025 | `W-ELAB-WIDTH-XZEXPAND` | Warn | MVP-SIM | X/Z value expanded to a wider target | Verilator WIDTHXZEXPAND |
| W3026 | `W-ELAB-BLOCKING-MIX` | Warn | MVP-SIM | Same var driven by both blocking and non-blocking | Verilator BLKANDNBLK (Error); IEEE 1800 §4 |
| W3027 | `W-ELAB-NBA-IN-COMB` | Warn | MVP-SIM | Non-blocking assignment in a combinational block | Verilator COMBDLY; IEEE 1800 §10.4.2 |
| W3028 | `W-ELAB-NBA-IN-INITIAL` | Warn | MVP-SIM | Non-blocking assignment in an initial/final block | Verilator INITIALDLY |
| W3029 | `W-ELAB-CASE-INCOMPLETE` | Warn | MVP-SIM | `case` no default and not all selector values covered | Verilator CASEINCOMPLETE |
| W3030 | `W-ELAB-CASE-OVERLAP` | Warn | MVP-SIM | Overlapping case items (later unreachable) | Verilator CASEOVERLAP |
| W3031 | `W-ELAB-CASE-WITH-X` | Warn | MVP-SIM | Plain `case` item contains a literal x/z bit | Verilator CASEWITHX |
| W3032 | `W-ELAB-LATCH` | Warn | MVP-SIM | Latch inferred in a combinational block | Verilator LATCH/NOLATCH |
| W3033 | `W-ELAB-IMPLICIT-STATIC` | Warn | MVP-SIM | Implicit static lifetime on a task/function var | Verilator IMPLICITSTATIC |
| W3034 | `W-ELAB-SELRANGE` | Warn | MVP-SIM | Constant bit/part-select provably out of range | Verilator SELRANGE; iverilog -Wselect-range |
| W3035 | `W-ELAB-CMP-CONST` | Warn | MVP-SIM | Comparison provably always true/false | Verilator CMPCONST |
| W3036 | `W-ELAB-UNSIGNED-CMP` | Warn | MVP-SIM | Unsigned comparison with constant result | Verilator UNSIGNED |
| W3037 | `W-ELAB-REAL-CONVERT` | Warn | MVP-SIM | Implicit real-to-integer conversion (precision loss) | Verilator REALCVT; IEEE 1800 §6.12.2 |
| W3038 | `W-ELAB-INFINITE-LOOP` | Warn | MVP-SIM | Statically-always-true loop with no exit | Verilator INFINITELOOP; iverilog -Winfloop (opt-in) |
| W3039 | `W-ELAB-PIN-MISSING` | Warn | MVP-SIM | Instance leaves a declared port unconnected | Verilator PINMISSING; iverilog -Wportbind; VCS TFIPC-L |
| W3041 | `W-ELAB-PORT-SHORT` | Warn | MVP-SIM | Module output port tied to a constant | Verilator PORTSHORT |
| W3042 | `W-ELAB-MULTITOP` | Warn | MVP-SIM | Multiple uninstantiated top modules | Verilator MULTITOP; IEEE 1800 §3.12 |
| W3043 | `W-ELAB-IGNORED-RETURN` | Warn | MVP-SIM | Non-void function called as a statement | Verilator IGNOREDRETURN |
| W3044 | `W-ELAB-NO-RETURN` | Warn | MVP-SIM | Non-void function never sets its return value | Verilator NORETURN |
| W3045 | `W-ELAB-NO-EFFECT` | Warn | MVP-SIM | Statement/expression has no observable effect | Verilator NOEFFECT |
| W3046 | `W-ELAB-ALWCOMBORDER` | Warn | MVP-SIM | `always_comb` reads a var before assigning it | Verilator ALWCOMBORDER |
| W3047 | `W-ELAB-ALWAYS-NEVER` | Warn | MVP-SIM | `always @*` with empty sensitivity never triggers | Verilator ALWNEVER |
| W3048 | `W-ELAB-SENS-ENTIRE-ARRAY` | Warn | MVP-SIM | `always @*` word-select pulls whole array into sens | iverilog -Wsensitivity-entire-array |
| W3049 | `W-ELAB-SENS-ENTIRE-VECTOR` | Warn | LINT | `always @*` part-select pulls whole vector into sens | iverilog -Wsensitivity-entire-vector (opt-in) |
| W3050 | `W-ELAB-FLOATING-NET` | Warn | LINT | Net present in design but has no drivers | iverilog -Wfloating-nets (opt-in) |
| W3052 | `W-LINT-DEFPARAM` | Warn | MVP-SIM | Deprecated `defparam` parameter override | Verilator DEFPARAM; IEEE 1364-2005 §12.2.1 |
| W3053 | `W-LINT-VAR-HIDDEN` | Warn | LINT | Variable shadows one in an enclosing scope | Verilator VARHIDDEN |
| W3054 | `W-LINT-UNUSED` | Warn | LINT | Signal/parameter/genvar unused or undriven | Verilator UNUSEDSIGNAL/UNDRIVEN/UNUSEDPARAM |
| W3055 | `W-LINT-STYLE-MISC` | Warn | LINT | Assorted off-by-default style issues (catch-all) | Verilator BLKSEQ/EOFNEWLINE/IMPORTSTAR/… |

### 4xxx · RUNTIME  (9)

| 번호 | mnemonic | sev | scope | 조건 | 출처 / 매핑 |
|---|---|---|---|---|---|
| E4008 | `E-RUN-DIV-ZERO` | Erro | MVP-SIM | Integer division or modulo by zero (result x) | IEEE 1364-2005 §5.1.5 |
| E4009 | `E-RUN-ILLEGAL-SCALAR-SELECT` | Erro | MVP-SIM | Bit/part-select of a scalar or real value | IEEE 1364-2005 §4.2.1 |
| I4015 | `I-RUN-STOP` | Info | MVP-SIM | `$stop` executed (simulation suspended) | IEEE 1800 §20.2; iverilog/vvp -n/-N |
| W4010 | `W-RUN-FORMAT-MISMATCH` | Warn | MVP-SIM | Format-specifier / argument count or type mismatch | IEEE 1364-2005 §17.1.1.2 |
| W4011 | `W-RUN-WAIT-CONST` | Warn | MVP-SIM | `wait` on a compile-time constant condition | Verilator WAITCONST |
| W4012 | `W-RUN-STMT-DELAY` | Warn | MVP-SIM | Procedural statement delay under limited delay model | Verilator STMTDLY |
| W4013 | `W-RUN-ZERO-DELAY` | Warn | MVP-SIM | `#0` zero delay (inactive-region scheduling) | Verilator ZERODLY; IEEE 1800 §15.4 |
| W4014 | `W-LINT-ASSIGN-DELAY` | Warn | LINT | Intra-assignment delay on a non-blocking assign | Verilator ASSIGNDLY (off-by-default) |
| W4017 | `W-RUN-UNIQUE-VIOLATION` | Warn | MVP-SIM | `unique`/`priority` case or if violation at runtime | IEEE 1800-2017 §12.5.3 (mandatory report) |

### 5xxx · ASSERTION / SVA (예약, Phase 2)  (3)

| 번호 | mnemonic | sev | scope | 조건 | 출처 / 매핑 |
|---|---|---|---|---|---|
| E5001 | `E-SVA-CONCURRENT-ASSERT-FAIL` | Erro | SVA | Concurrent assertion property fails (default $error) | IEEE 1800-2017 §16.5/§16.3 |
| W5002 | `W-SVA-ASSUME-COVER` | Warn | SVA | `assume` fails or `cover` property never hit | IEEE 1800-2017 §16.12/§16.13 |
| W5004 | `W-SVA-PAST-DEPTH` | Warn | SVA | `$past` delay exceeds practical depth | Verilator TICKCOUNT; IEEE 1800 §16.9 |

### 6xxx · SV-TYPE (예약)  (7)

| 번호 | mnemonic | sev | scope | 조건 | 출처 / 매핑 |
|---|---|---|---|---|---|
| E6001 | `E-TYPE-ENUM-VALUE` | Erro | SV-TYPE | Enum assigned a non-member value without a cast | Verilator ENUMVALUE; IEEE 1800 §6.19 |
| E6002 | `E-TYPE-ENUM-ITEM-WIDTH` | Erro | SV-TYPE | Enum item value does not fit the enum base width | Verilator ENUMITEMWIDTH |
| E6003 | `E-TYPE-CONST-WRITTEN` | Erro | SV-TYPE | Assignment to a `const` after initialization | Verilator CONSTWRITTEN |
| E6004 | `E-TYPE-CAST-FAILURE` | Erro | SV-TYPE | Dynamic `$cast` failure | IEEE 1800-2017 §6.24.2; Verilator CASTCONST |
| E6006 | `E-TYPE-CLASS-RULE` | Erro | SV-TYPE | SystemVerilog class/OOP rule violation | Verilator ENCAPSULATED/LIFETIME/… |
| W6005 | `W-TYPE-RANDOM-LIMIT` | Warn | SV-TYPE | Constrained-random/coverage unsupported or unsat | Verilator CONSTRAINTIGN/COVERIGN/RANDC |
| W6007 | `W-TYPE-REAL-CONVERT` | Warn | SV-TYPE | Real-to-integer conversion in typed context (dup of W3037) | Verilator REALCVT; IEEE 1800 §6.12.2 |

### 7xxx · VHDL (예약, Phase 3)  (9)

> Phase-3 설계 주의: VHDL의 bound-check/overflow는 **중단(Fatal)** 이지만 Verilog 범위 초과는
> x를 읽고 **계속**한다 — `E-RUN-RANGE` 의미를 VHDL에 재사용하지 말 것.

| 번호 | mnemonic | sev | scope | 조건 | 출처 / 매핑 |
|---|---|---|---|---|---|
| E7001 | `E-VHDL-NOT-DECLARED` | Erro | VHDL | VHDL name has no visible declaration | GHDL 'no declaration for'; IEEE 1076 |
| E7002 | `E-VHDL-UNIT-NOT-FOUND` | Erro | VHDL | VHDL design unit not found in library | GHDL 'unit not found in library' |
| E7003 | `E-VHDL-DUP-DECLARATION` | Erro | VHDL | Identifier already used in the declarative region | GHDL 'identifier already used' |
| E7004 | `E-VHDL-TYPE-MISMATCH` | Erro | VHDL | Type incompatibility / association failure | GHDL type/association errors |
| E7009 | `E-VHDL-UNRESOLVED-MULTIDRIVER` | Erro | VHDL | Multiple drivers on an unresolved-type signal | GHDL resolution-function enforcement |
| F7005 | `F-VHDL-ASSERTION-FAILURE` | Fata | VHDL | `assert`/`report` at/above the stopping severity | GHDL --assert-level; IEEE 1076 §8.2 |
| F7006 | `F-VHDL-BOUND-CHECK` | Fata | VHDL | Runtime constraint (bound-check) failure | GHDL 'bound check failure' |
| F7007 | `F-VHDL-OVERFLOW` | Fata | VHDL | Arithmetic overflow (CONSTRAINT_ERROR) | GHDL 'overflow' |
| W7008 | `W-VHDL-METAVALUE` | Warn | VHDL | NUMERIC_STD metavalue detected in conversion | GHDL --ieee-asserts |

### 제외 (구현 범위 밖)

순수 synthesis-only / 컴파일드-모델 아티팩트는 코드를 부여하지 않았다: Verilator
GENCLK(5.000 이후 미발생) · SYMRSVDWORD(C++ 키워드 충돌 — vitamin은 C++ codegen 없음) ·
NEEDTIMINGOPT/NOTIMING(`--timing` opt-in) · UNOPTFLAT(컴파일드 정적 스케줄 성능 — 실제 조합
루프는 `F-RUN-NO-CONVERGE`가 커버) · BLKLOOPINIT/UNOPTTHREADS/HIERBLOCK 등 멀티스레드-빌드
진단(인터프리터에 해당 없음).

---

## Sources

- [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) — severity lattice · MsgCode 체계 ·
  게이트 · exit 코드 · RTL severity 통합 (본 카탈로그의 상위 설계)
- [14-staged-artifacts.md](14-staged-artifacts.md) — FLIST/ART 코드의 hash·staleness·filelist 의미
- [09-testing-and-verification.md](09-testing-and-verification.md) — corpus가 코드로 assert,
  exit 분류
- hdl-reference/system-tasks/04-simulation-control.md · 13-misc.md · 01-display-io.md;
  systemverilog/07-assertions-sva.md — `$info`/`$warning`/`$error`/`$fatal`/assert severity
- IEEE 1800-2017 §16(assertions) §20.10–20.12(severity/elaboration tasks) §22(preprocess) ·
  IEEE 1364-2005 §19
- **부록 A 공식 출처:** Verilator 경고 목록 https://verilator.org/guide/latest/warnings.html ·
  Icarus Verilog `-W` 플래그(https://steveicarus.github.io/iverilog/usage/command_line_flags.html) ·
  Synopsys VCS / Cadence Xcelium 메시지 클래스 · GHDL 진단(VHDL Phase 3 예약,
  https://ghdl.github.io/ghdl/) · IEEE 1800-2017 §11(연산자/폭)·§12(case/generate)·§13(tasks/functions)·
  §6(types) · IEEE 1364-2005 §5·§12 · IEEE 1076-2008(VHDL)
