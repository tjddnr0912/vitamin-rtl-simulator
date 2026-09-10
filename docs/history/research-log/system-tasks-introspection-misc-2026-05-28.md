---
title: "SystemVerilog System Functions 조사 — Introspection · Misc (Phase 2)"
date: 2026-05-28
author: research-skill (Claude Sonnet 4.6)
scope:
  - IEEE 1800-2017 §20.6 (type/data query: $typename, $bits, $isunbounded)
  - IEEE 1800-2017 §20.7 (array dimension query: $size, $left, $right, $low, $high, $increment, $dimensions)
  - IEEE 1800-2017 §20.5 ($cast dynamic type check)
  - IEEE 1800-2017 §20.10 ($value$plusargs, $test$plusargs)
  - IEEE 1800-2017 §20.11 (elaboration severity tasks: $fatal, $error, $warning, $info)
  - IEEE 1800-2017 §21.3 ($system shell execution)
  - $exit — program block phase control
  - Icarus Verilog / Verilator 동작 차이
rounds: 2
status: complete
---

# SystemVerilog System Functions 조사 — Introspection · Misc

## 조사 배경

Vitamin RTL 시뮬레이터 `hdl-builtins` 크레이트의 Phase 2 구현을 위해
타입 조회·배열 차원 쿼리 함수 카테고리(§20.6/§20.7)와
테스트 인프라용 잡함수 카테고리(plusargs, 심각도 태스크, $exit, $system)의
표준 의미론과 주요 시뮬레이터 지원 현황을 확인한다.

---

## A. 타입·배열 조회 함수

### $typename(expr_or_type)

- **표준**: IEEE 1800-2017 §20.6
- 인자로 넘긴 표현식 또는 타입의 "해결된 타입 이름"을 문자열로 반환한다.
- typedef, enum, parameterized type 모두 처리된다.
- 주요 반환값 패턴:
  - `logic [7:0]` 변수 → `"logic [7:0]"`
  - enum typedef `pkt_t` → `"pkt_t"` (typedef 이름 보존)
  - 직접 기술한 `enum logic [1:0] {A, B}` 타입 → 내부 표현 문자열
- 디버그·로깅 목적으로만 쓰인다; 반환 문자열을 파싱해 로직을 분기하는 것은 표준이 권장하지 않는다.
- **Icarus**: 부분 지원 — 기본 타입은 동작, typedef 처리 불완전
- **Verilator**: 지원 (`--sv` 모드)

[출처: circuitcove.com/system-tasks-query/ WebFetch ✓;
chipverify.com quick-refresher WebFetch partial ✓]

### $cast(dest_var, src_expr)

- **표준**: IEEE 1800-2017 §20.5
- 런타임 타입 체크를 수행하는 동적 캐스트.
- 두 가지 호출 형태:

  **태스크(task) 형태**: 캐스트 실패 시 런타임 에러를 발생시키고 dest_var를 건드리지 않는다.
  ```sv
  $cast(dest_handle, src_handle);
  ```

  **함수(function) 형태**: 성공이면 1, 실패이면 0을 반환한다. 실패 시 dest_var는 **변경되지 않는다**. 런타임 에러는 발생하지 않는다.
  ```sv
  if (!$cast(dest_handle, src_handle))
    $error("cast failed at %0t", $time);
  ```

- 주요 사용처:
  - 클래스 계층 다운캐스트: 부모 핸들이 자식 객체를 가리키는지 런타임에 확인
  - enum 캐스트: 정수값이 열거값 범위 안에 있는지 런타임에 확인
- 중요한 점: `$cast`는 **값(value)**을 검사하는 것이지 변수의 선언 타입만을 보는 것이 아니다. 같은 코드도 런타임 값에 따라 결과가 달라진다.
- 정적 캐스트(`type'(expr)`)와 구별: 정적은 컴파일타임 체크, 동적은 런타임 체크.
- **Icarus**: 클래스 OOP 지원이 제한적이므로 class hierarchy cast는 제한됨; enum cast는 부분 지원
- **Verilator**: 지원 (class OOP 포함)

[출처: vlsiverify.com systemverilog-casting/ WebFetch ✓;
siemens verification-horizons "$cast() runtime checks" WebFetch ✓]

### $isunbounded(expr)

- **표준**: IEEE 1800-2017 §20.6
- 인자로 넘긴 표현식이 "unbounded" 값(`$`)이면 1'b1, 아니면 1'b0을 반환한다.
- 주요 사용처: 파라미터화 모듈에서 파라미터가 `$`로 설정되었는지 확인해 분기 처리

  ```sv
  module fifo #(parameter int DEPTH = $) (...);
    if (!$isunbounded(DEPTH) && DEPTH < 4)
      $fatal(1, "DEPTH must be >= 4 or unbounded ($)");
  ```

- SVA unbounded range (`##[n:$]`) 여부 확인에도 사용.
- **Icarus**: 지원
- **Verilator**: 지원

[출처: circuitcove.com/system-tasks-query/ WebFetch ✓]

---

### 배열 차원 쿼리 함수군 ($size, $left, $right, $low, $high, $increment, $dimensions)

**표준**: IEEE 1800-2017 §20.7

이 함수들은 모두 선택적 `dim` 인자를 받는다. dim의 번호 체계:
- **dim=1**: 가장 왼쪽 unpacked 차원
- 이후 오른쪽 방향으로 unpacked 차원 순서대로 번호 증가
- unpacked 다 지나면 왼쪽에서 오른쪽 방향으로 packed 차원
- dim 생략 시 기본값 = 1 (첫 번째 unpacked 차원)

예시 선언:
```sv
logic [7:0] [15:0] packed_arr [2:0][3:0];
//  unpacked: dim1=[2:0], dim2=[3:0]
//  packed:   dim3=[7:0], dim4=[15:0]
```

#### $size(arr [, dim])

- 지정 차원의 원소 수를 반환. `$high(arr,dim) - $low(arr,dim) + 1`과 같다.
- 동적 배열(`[]`)은 현재 할당 크기를 반환한다.
- 연관 배열(`[type]`)은 지원하지 않는다 — 런타임 항목 수는 `.num()` 메서드로 조회.
- 큐(`[$]`)는 현재 원소 수를 반환.

#### $left(arr [, dim]) / $right(arr [, dim])

- 선언된 **좌/우 경계**를 반환한다. `[3:0]`이면 left=3, right=0.
- "왼쪽이 크다" 방향(down-counting)과 "오른쪽이 크다" 방향(up-counting) 모두 올바르게 반영.

#### $low(arr [, dim]) / $high(arr [, dim])

- **최솟값/최댓값** 경계를 반환한다. 방향(left/right)에 무관하게 항상 작은쪽이 $low, 큰쪽이 $high.
- `[0:7]`이면: left=0, right=7, low=0, high=7.
- `[7:0]`이면: left=7, right=0, low=0, high=7.

#### $increment(arr [, dim])

- left ≥ right이면 1, left < right이면 -1을 반환.
- 인덱스 방향 감지에 유용: for 루프에서 방향 독립적 순회를 구현할 때.

#### $dimensions(arr) / $unpacked_dimensions(arr)

- `$dimensions`: packed + unpacked 전체 차원 수. 1-D scalar bit vector는 1 반환.
- `$unpacked_dimensions`: unpacked 차원 수만. packed array는 0.
- 비-배열 타입에 대해 0 반환.

**예시**:
```sv
logic [7:0] byte_arr [0:3];  // unpacked 1차원, packed 1차원
$dimensions(byte_arr)         // = 2
$unpacked_dimensions(byte_arr) // = 1
$size(byte_arr)               // = 4 (unpacked dim)
$size(byte_arr, 2)            // = 8 (packed dim)
$left(byte_arr, 1)            // = 0
$high(byte_arr, 1)            // = 3
```

**Icarus/Verilator 지원**:
| 함수 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$size` | 지원 (단일 dim) | 지원 |
| `$left/$right/$low/$high` | 부분 지원 | 지원 |
| `$increment` | 제한적 | 지원 |
| `$dimensions/$unpacked_dimensions` | 제한적 | 지원 |

Icarus는 SV 배열 차원 쿼리의 서브셋만 구현하므로 복잡한 멀티-dim 쿼리에서 차이가 있을 수 있다.

[출처: circuitcove.com/system-tasks-query/ WebFetch ✓;
vlsi.pro/system-verilog/array-querying-system-functions/ WebFetch ✓]

---

## B. 기타 (Misc) 함수·태스크

### $test$plusargs(string) 과 $value$plusargs(string, var)

**표준**: IEEE 1800-2017 §20.10

시뮬레이터 커맨드라인에 `+`로 시작하는 plusarg를 넘기고,
시뮬레이션 코드 안에서 그 값을 읽어오는 메커니즘.

#### $test$plusargs(user_string)

- 공급된 plusarg 중 하나라도 `user_string`으로 시작하면 non-zero(1), 없으면 0 반환.
- **prefix 매칭**: `+STANDBY`라는 arg는 `"STAND"`, `"S"`, `"STANDBY"` 모두에 매칭된다.
- 대소문자 구분(case-sensitive).
- 값이 필요 없는 boolean flag에 적합.

```sv
if ($test$plusargs("VERBOSE")) begin
  $display("[DEBUG] verbose mode on");
end
```

커맨드라인: `vvp sim.vvp +VERBOSE` (Icarus), `./sim +VERBOSE` (Verilator)

#### $value$plusargs(user_string, variable)

- format 지정자를 포함한 문자열로 매칭하고, 매칭되면 값을 `variable`에 저장한다.
- 매칭·저장 성공이면 non-zero(1), 실패이면 0. **실패 시 variable은 변경되지 않는다.**
- format 문자열 형식: `"ARGNAME=%fmt"` (이름, `=`, format specifier 사이에 공백 없음)

| format code | 변환 타입 |
|-------------|---------|
| `%d` | 10진수 정수 |
| `%o` | 8진수 정수 |
| `%h` / `%x` | 16진수 정수 |
| `%b` | 2진수 정수 |
| `%e` | 실수 (지수 표기) |
| `%f` | 실수 (소수 표기) |
| `%g` | 실수 (소수 또는 지수 — 더 짧은 쪽) |
| `%s` | 문자열 (변환 없음) |

```sv
int    seed;
string testname;
real   timeout;

if ($value$plusargs("SEED=%d",    seed))    $display("seed=%0d", seed);
if ($value$plusargs("TEST=%s",    testname)) $display("test=%s",  testname);
if ($value$plusargs("TIMEOUT=%f", timeout))  $display("timeout=%f", timeout);
```

커맨드라인 예: `+SEED=42 +TEST=axi_burst +TIMEOUT=100.5`

- format mismatch(예: 문자열에 `%d`를 쓰면)는 런타임 에러 또는 구현 정의 동작이다.
- `+STRING=` 처럼 빈 값도 허용된다 (`%s`는 빈 문자열로 수신).

**Icarus/Verilator**: 두 함수 모두 완전 지원.

[출처: chipverify.com/systemverilog/systemverilog-command-line-input WebFetch ✓;
theartofverification.com/plusargs-in-systemverilog/ WebFetch ✓]

---

### $system("command")

**표준**: IEEE 1800-2017 §21.3 (구현체에 따라 §21.4까지 연관)

- 인자 문자열을 OS 쉘에 전달해 실행한다.
- **함수 형태**: 셸 함수의 반환값(int)을 반환한다. POSIX에서는 exit status.
- **태스크 형태**: 반환값 무시.

```sv
int ret;
ret = $system("cp golden.mem /tmp/golden.mem");
if (ret != 0) $error("file copy failed: %0d", ret);
$system("mkdir -p /tmp/sim_out");
```

**보안·포터빌리티 주의사항**:
1. OS 의존성 — Windows와 Linux/macOS에서 사용 가능한 명령이 다르다.
   `ls`, `cp`, `mkdir` 등은 Windows cmd.exe에서 동작하지 않는다.
2. 경로 인젝션 리스크 — plusargs 등에서 받은 문자열을 그대로 넘기면 임의 명령 실행 위험.
3. 시뮬레이션 환경에서 파생된 프로세스는 시뮬레이터의 file descriptor를 상속할 수 있다.
4. **테스트 인프라 전용**으로 한정해야 한다. RTL 시뮬레이션 로직에서 사용하는 것은 안티패턴.

**Icarus**: 지원 (`$finish_and_return`은 별도 extension).
**Verilator**: 지원 (Linux/macOS 환경에서 정상 동작).

[출처: systemverilog.io/verification/ten-utilities/ WebFetch — $system 단편 ✓]

---

### 심각도 태스크: $fatal / $error / $warning / $info

**표준**: IEEE 1800-2017 §20.11 (elaboration 컨텍스트) + §20.12 (simulation 컨텍스트)

같은 이름의 태스크가 **두 컨텍스트**에서 동작한다:

| 컨텍스트 | 실행 시점 | 제어 구문 |
|---------|---------|---------|
| **Elaboration** | 모듈 파라미터 체크, generate 블록 | procedural 코드 **외부** |
| **Simulation** | 런타임 assertion, 테스트 로직 | procedural 코드 **내부** |

코드가 procedural 블록 안에 있으면 자동으로 simulation-time 동작을 한다.

**시그니처**:
```sv
$fatal   [(finish_number [, list_of_arguments])];
$error   [(list_of_arguments)];
$warning [(list_of_arguments)];
$info    [(list_of_arguments)];
```

- `finish_number` ($fatal 전용): 0, 1, 2 — Verilog `$finish`의 `finish_number`와 동일.
  - 0: 아무것도 출력하지 않음
  - 1: 시뮬레이션 시간 출력 (기본)
  - 2: 시뮬레이션 시간 + 프로세스 메모리 사용량 출력 (구현 정의)
- `list_of_arguments`: `$display`와 동일한 format string 문법.

**각 태스크의 동작**:

| 태스크 | Elaboration 동작 | Simulation 동작 |
|--------|----------------|----------------|
| `$fatal` | elaboration 즉시 중단, 시뮬레이션 생성 없음 | 시뮬레이션 런타임 에러 → 강제 종료 |
| `$error` | 에러 메시지 출력, elaboration 계속 | 에러 메시지 출력, 시뮬레이션 계속 |
| `$warning` | 경고 메시지 출력, 억제 가능 | 경고 메시지 출력, 억제 가능 |
| `$info` | 정보 메시지 출력 | 정보 메시지 출력 |

**모든 태스크의 공통 출력 포맷** (툴이 자동 추가):
- 파일명 + 줄 번호
- 호출된 scope의 계층 경로
- Simulation 컨텍스트이면 시뮬레이션 시간

SVA action block에서도 사용 가능:
```sv
assert property (@(posedge clk) req |-> ack)
  else $error("req asserted but ack never came, req=%0b at %0t", req, $time);
```

Elaboration 컨텍스트 사용 예:
```sv
module core #(parameter int WIDTH = 8) ();
  if (WIDTH < 4 || WIDTH > 64)
    $fatal(1, "WIDTH=%0d out of valid range [4,64]", WIDTH);
endmodule
```

**Icarus**: `$error`/`$warning`/`$info` 지원. `$fatal` elaboration-time 지원은 부분적 (시뮬레이션 타임으로 fallback 가능). Verilator issue #1429에서 elaboration tasks가 `closed/fixed`로 해결됨.
**Verilator**: elaboration tasks 공식 지원 (issue #1429 fix 이후).

[출처: accellera.org/eda/sv-bc/att-5678/severity_tasks_3.htm WebFetch ✓;
circuitcove.com/system-tasks-simulation-control/ WebFetch ✓;
github.com/verilator/verilator/issues/1429 WebFetch ✓;
cocotb issue #107 (Overload severity tasks) — 교차 확인 ✓]

---

### $exit

**표준**: IEEE 1800-2017 §21 (program block phase control)

- `$exit`은 모든 **program block**이 완료될 때까지 기다렸다가 `$finish`를 호출한다.
- `$finish`와의 차이: `$finish`는 즉시 시뮬레이션을 종료하지만, `$exit`은 테스트 종료 신호만 발행하고 실행 중인 program block이 깔끔하게 마무리되도록 기다린다.
- UVM 또는 클래스 기반 테스트벤치에서 `run_phase()` 등이 끝난 뒤 시뮬레이션을 종료할 때 관용적으로 사용.

```sv
program automatic test;
  initial begin
    // ... test logic ...
    $exit;   // program block 완료 후 $finish 호출
  end
endprogram
```

**Icarus**: `program` 블록 지원이 제한적 → `$exit` 동작도 제한적.
**Verilator**: `program` 블록 미지원 → `$exit` 사용 불가. `$finish`를 직접 사용해야 한다.

[출처: circuitcove.com/system-tasks-simulation-control/ WebFetch ✓]

---

## Sources

1. **circuitcove.com — Data and Array Query Functions** — https://circuitcove.com/system-tasks-query/ (WebFetch Round 1 ✓)
2. **circuitcove.com — Simulation Control Tasks** — https://circuitcove.com/system-tasks-simulation-control/ (WebFetch Round 2 ✓)
3. **vlsiverify.com — SystemVerilog Casting** — https://vlsiverify.com/system-verilog/systemverilog-casting/ (WebFetch Round 2 ✓)
4. **siemens verificationhorizons — $cast() runtime checks** — https://blogs.sw.siemens.com/verificationhorizons/2021/06/28/runtime-checks-with-the-cast-method/ (WebFetch Round 2 ✓)
5. **chipverify.com — Command Line Input** — https://chipverify.com/systemverilog/systemverilog-command-line-input (WebFetch Round 1 ✓)
6. **theartofverification.com — Plusargs** — https://theartofverification.com/plusargs-in-systemverilog/ (WebFetch Round 1 ✓)
7. **accellera.org sv-bc — Severity Tasks** — https://www.accellera.org/images/eda/sv-bc/att-5678/severity_tasks_3.htm (WebFetch Round 2 ✓)
8. **vlsi.pro — Array Querying System Functions** — https://vlsi.pro/system-verilog/array-querying-system-functions/ (WebFetch Round 2 ✓)
9. **systemverilog.io — Ten Utilities ($system)** — https://www.systemverilog.io/verification/ten-utilities/ (WebFetch Round 1 partial ✓)
10. **github.com/verilator/verilator/issues/1429** — Elaboration tasks support (WebFetch Round 2 — closed/fixed ✓)
11. **steveicarus.github.io — Icarus Extensions** — https://steveicarus.github.io/iverilog/usage/icarus_verilog_extensions.html (WebFetch Round 2 ✓)
12. **IEEE 1800-2017 §20.5** ($cast), **§20.6** ($typename, $isunbounded, $bits), **§20.7** (array dim query), **§20.10** (plusargs), **§20.11** (elaboration severity tasks), **§21.3** ($system) — 2차 소스 교차 확인 (직접 fetch 불가)
