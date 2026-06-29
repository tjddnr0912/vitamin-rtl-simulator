# 13 · 기타 태스크 (Plusargs · System · Severity · Exit)

## 개요

이 카테고리는 테스트 인프라를 구성하는 네 가지 유틸리티 그룹을 다룬다.

- **Plusargs** (`$test$plusargs`, `$value$plusargs`): 커맨드라인 인자를 시뮬레이션 코드로 전달하는 채널
- **Shell 실행** (`$system`): OS 셸 명령을 시뮬레이션 안에서 호출
- **심각도 태스크** (`$fatal`, `$error`, `$warning`, `$info`): elaboration/simulation 양쪽에서 사용하는 구조화된 진단 출력
- **테스트 종료** (`$exit`): program block 기반 테스트벤치의 단계 제어

## 구현 상태

- ✅ **구현됨**: `$test$plusargs`, `$value$plusargs`(format 코드 파서 + ref-VAR 쓰기 statement-level 인터셉트, format_version 7), `$fatal`, `$error`, `$warning`, `$info`(severity 태스크 — `$fatal`=exit1, elaboration/simulation 양 컨텍스트), `$exit`(v9 — `$finish` 별칭으로 배선: `SysTaskId::Finish`).
- ⏳ **미구현**: `$system`.
  - `$system`은 `SysFuncId`/`SysTaskId` 어디에도 미배선이다. **함수 형태**(`ret = $system(...)`)는 expression 경로에서 `E3009 "unsupported system function"`로 loud-reject되고, **태스크 형태**(`$system("...")`)는 unknown-task 경로라 `W-ELAB-FEATURE-LIMIT` 경고 후 스킵된다(no-op, 테스트벤치 생존).

---

## Plusargs — 커맨드라인에서 값 읽기

### `$test$plusargs(user_string)` — boolean flag 감지

- **표준**: IEEE 1800-2017 §20.10
- 커맨드라인에 공급된 plusarg 중 하나라도 `user_string`으로 시작하면
  non-zero(1)을 반환하고, 없으면 0을 반환한다.
- **prefix 매칭** 방식이다: `+VERBOSE_MODE`라는 arg는
  `"V"`, `"VERBOSE"`, `"VERBOSE_MODE"` 모두에 매칭된다.
- **대소문자 구분(case-sensitive)**.
- 값이 필요 없는 boolean flag 활성화에 적합하다.

```sv
// 시뮬레이션 코드
initial begin
  if ($test$plusargs("VERBOSE"))
    $display("[DEBUG] verbose logging enabled");
  if ($test$plusargs("WAVE"))
    $dumpvars();
end
```

커맨드라인:
```sh
# Icarus Verilog
vvp sim.vvp +VERBOSE +WAVE

# Verilator
./obj_dir/Vtop +VERBOSE +WAVE
```

---

### `$value$plusargs(user_string, variable)` — 값 추출

- **표준**: IEEE 1800-2017 §20.10
- format 지정자를 포함한 문자열로 매칭하고, 매칭되면 값을 `variable`에 저장한다.
- 매칭·저장 성공이면 non-zero(1), 실패이면 0. **실패 시 variable은 변경되지 않는다.**
- format 문자열 형식: `"ARGNAME=%fmt"` — 이름, `=`, format code 사이에 공백 없음.

#### 지원 format code

| code | 변환 |
|------|------|
| `%d` | 10진수 정수 |
| `%o` | 8진수 정수 |
| `%h` / `%x` | 16진수 정수 |
| `%b` | 2진수 정수 |
| `%e` | 실수 (지수 표기) |
| `%f` | 실수 (소수 표기) |
| `%g` | 실수 (소수 또는 지수 — 더 짧은 쪽 선택) |
| `%s` | 문자열 (변환 없음) |

```sv
int    seed;
string testname;
real   timeout_ns;
logic  [7:0] mask;

// 각 plusarg 독립적으로 조회
if ($value$plusargs("SEED=%d",    seed))
  $display("seed=%0d", seed);
if ($value$plusargs("TEST=%s",    testname))
  $display("test=%s", testname);
if ($value$plusargs("TIMEOUT=%f", timeout_ns))
  $display("timeout=%.1f ns", timeout_ns);
if ($value$plusargs("MASK=%h",    mask))
  $display("mask=0x%02h", mask);
```

커맨드라인:
```sh
./sim +SEED=42 +TEST=axi_burst +TIMEOUT=1000.0 +MASK=ff
```

**edge case 메모**:
- `+KEY=` (빈 값): `%s`는 빈 문자열로 수신, 정수 format에서는 구현 정의.
- format mismatch (예: 정수가 아닌 문자열에 `%d`): 런타임 에러 또는 구현 정의 동작.
- 같은 plusarg name이 여러 번 공급되면 마지막 값이 사용된다 (구현 정의 포함).

**Icarus / Verilator**: 완전 지원.

---

## Shell 실행 — `$system("command")` ⏳ 미구현

> **vitamin 구현 상태**: `$system`은 `SysFuncId`/`SysTaskId` 어디에도 미배선이다.
> **함수 형태**(반환값 사용)는 expression 경로에서 `E3009 "unsupported system function"`로
> loud-reject되고, **태스크 형태**(`$system("...")`)는 unknown-task 경로라
> `W-ELAB-FEATURE-LIMIT` 경고 후 스킵된다(no-op). 아래는 IEEE 참조 문서다.

- **표준**: IEEE 1800-2017 §21.3
- 인자 문자열을 OS 셸에 전달해 실행한다.
- **함수 형태**: 셸 프로세스의 종료 코드(exit status)를 int로 반환한다.
  POSIX 환경에서 0 = 성공, non-zero = 실패.
- **태스크 형태**: 반환값을 무시.

```sv
// 함수 형태 — 반환값 체크
int ret;
ret = $system("cp golden.mem /tmp/golden.mem");
if (ret != 0)
  $error("file copy failed with code %0d", ret);

// 태스크 형태
$system("mkdir -p /tmp/sim_out");
$system("date > /tmp/sim_out/timestamp.txt");
```

**보안·이식성 주의사항**:

1. **OS 의존성**: `ls`, `cp`, `mkdir` 같은 POSIX 명령은 Windows cmd.exe에서 동작하지 않는다.
   크로스-플랫폼 테스트 환경을 고려한다면 Makefile이나 쉘 래퍼로 추상화할 것.

2. **커맨드 인젝션 리스크**: plusargs 등 외부 입력에서 받은 문자열을
   그대로 `$system`에 전달하면 임의 명령 실행이 가능해진다.
   항상 리터럴 문자열 또는 충분히 검증된 값만 전달할 것.

3. **파일 디스크립터 상속**: 파생 프로세스가 시뮬레이터의 파일 디스크립터를 상속할 수 있다.

4. **용도 제한**: `$system`은 테스트 인프라 전용이다.
   RTL 시뮬레이션 로직이나 assertion에서 사용하는 것은 안티패턴.

**Icarus**: 지원.
**Verilator**: 지원 (Linux/macOS 기준 정상 동작).

---

## 심각도 태스크 — `$fatal` / `$error` / `$warning` / `$info`

- **표준**: IEEE 1800-2017 §20.11 (elaboration), §20.12 (simulation assertion context)

이 네 태스크는 **같은 이름으로 두 컨텍스트**에서 다르게 동작한다.

| 컨텍스트 | 실행 시점 | 위치 |
|---------|---------|------|
| **Elaboration** | 모듈 파라미터 검증, generate 블록 | procedural 코드 **외부** |
| **Simulation** | 런타임 assertion, 테스트 로직 | procedural 코드 **내부** |

코드가 procedural 블록(`initial`/`always`/`task` 등) 안에 있으면
자동으로 simulation-time 동작을 한다.

### 시그니처

```sv
$fatal   [(finish_number [, list_of_arguments])];
$error   [(list_of_arguments)];
$warning [(list_of_arguments)];
$info    [(list_of_arguments)];
```

- `finish_number` (`$fatal` 전용): Verilog `$finish`의 finish_number와 동일.
  - `0`: 진단 출력 없음
  - `1`: 시뮬레이션 시간 출력 (기본값)
  - `2`: 시뮬레이션 시간 + 메모리 사용량 (구현 정의)
- `list_of_arguments`: `$display`와 동일한 format string 문법 (`%0d`, `%s`, `%0t` 등).

### 각 태스크의 동작

| 태스크 | Elaboration | Simulation |
|--------|-------------|------------|
| `$fatal` | elaboration 즉시 중단, 시뮬레이션 생성 없음 | 런타임 에러 → 시뮬레이션 강제 종료 |
| `$error` | 에러 출력 후 elaboration 계속 | 에러 출력 후 시뮬레이션 계속 |
| `$warning` | 경고 출력, 툴별 억제 가능 | 경고 출력, 툴별 억제 가능 |
| `$info` | 정보 출력 | 정보 출력 |

### 자동 추가되는 출력 정보

모든 심각도 태스크는 툴이 자동으로 다음 정보를 포함한 메시지를 생성한다:
- 파일명과 줄 번호
- 호출된 scope의 계층 경로 (예: `tb.dut.alu`)
- Simulation 컨텍스트이면 시뮬레이션 시간

### Elaboration 컨텍스트 — 파라미터 유효성 검사

generate 블록이나 모듈 레벨에서 파라미터를 검증하는 것이 대표적 패턴이다:

```sv
module mac #(
  parameter int IN_W  = 8,
  parameter int OUT_W = 16,
  parameter int LATENCY = 2
) (...);

  // 파라미터 범위 검사 — elaboration time에 실행
  if (IN_W < 1 || IN_W > 64)
    $fatal(1, "IN_W=%0d out of range [1,64]", IN_W);
  if (OUT_W < IN_W * 2)
    $fatal(1, "OUT_W=%0d too small for IN_W=%0d", OUT_W, IN_W);
  if (LATENCY < 1)
    $warning("LATENCY=%0d is unusually small", LATENCY);

endmodule
```

위 `$fatal`/`$warning`은 procedural 블록 밖에 있으므로 elaboration-time에 실행된다.
파라미터가 유효하지 않으면 시뮬레이션 실행 파일 자체가 생성되지 않는다.

### Simulation 컨텍스트 — assertion 및 테스트 로직

```sv
// SVA action block
assert property (@(posedge clk) req |-> ##[1:5] ack)
  else $error("ack missing after req at time %0t, req=%0b", $time, $sampled(req));

// procedural 코드
initial begin
  if (dut_result !== expected)
    $fatal(1, "MISMATCH: got=%0h expected=%0h at %0t",
           dut_result, expected, $time);
  else
    $info("test PASSED");
end
```

**Icarus**: `$error`/`$warning`/`$info` 지원.
Elaboration-time `$fatal`은 부분 지원 — simulation-time으로 fallback 가능.
**Verilator**: elaboration tasks 공식 지원 (issue #1429 fix 이후).

---

## 테스트 종료 — `$exit` ✅ 구현 (`$finish` 별칭)

> **vitamin 구현 상태**: `$exit`은 `$finish`의 별칭으로 배선됐다(v9 — `SysTaskId::Finish`).
> `program` 블록 자체는 미지원이므로 program-완료-대기 시맨틱은 단순화돼 즉시 `$finish` 동작이다
> (module-based TB에서 안전; Verilator도 동일하게 `$exit`=`$finish` 별칭).

- **표준**: IEEE 1800-2017 §21 (program block phase control)
- 모든 **program block**이 완료될 때까지 기다렸다가 `$finish`를 호출한다.

`$finish`는 즉시 시뮬레이션을 종료한다. 반면 `$exit`은 종료 신호만 발행하고,
실행 중인 program block들이 자체 정리(cleanup, coverage 집계 등)를 마칠 때까지 기다린다.
UVM / OVM 또는 클래스 기반 테스트벤치에서 테스트가 자연스럽게 마무리될 수 있도록
관용적으로 사용된다.

```sv
program automatic test;
  initial begin
    // 테스트 시퀀스
    fork
      run_stimulus();
      check_outputs();
    join

    // 모든 program block 완료 대기 후 $finish 호출
    $exit;
  end
endprogram
```

`$finish`와의 비교:

| | `$finish` | `$exit` |
|-|-----------|---------|
| 종료 시점 | 즉시 | 모든 program block 완료 후 |
| program block 정리 | 없음 | 있음 |
| 주요 사용 환경 | module-based TB | program-based TB |

**Icarus**: `program` 블록 지원이 제한적 → `$exit` 동작도 제한적.
**Verilator**: `program` 블록 미지원 → `$exit` 사용 불가.
Verilator 기반 테스트벤치에서는 `$finish`를 직접 사용한다.

---

## 함수·태스크 정리

| 이름 | 반환 | 용도 |
|------|------|------|
| `$test$plusargs(str)` | int (0/non-0) | boolean flag 커맨드라인 감지 |
| `$value$plusargs(str, var)` | int (0/non-0) | 값 포함 plusarg 읽기 |
| `$system(cmd)` | int (exit code) | OS 셸 명령 실행 |
| `$fatal(n, ...)` | — | 즉시 종료 (elaboration/simulation) |
| `$error(...)` | — | 에러 출력, 계속 실행 |
| `$warning(...)` | — | 경고 출력, 억제 가능 |
| `$info(...)` | — | 정보 출력 |
| `$exit` | — | program block 완료 후 $finish |

---

## Icarus / Verilator 지원

| 태스크 | Icarus Verilog | Verilator |
|--------|---------------|-----------|
| `$test$plusargs` | 완전 지원 | 완전 지원 |
| `$value$plusargs` | 완전 지원 | 완전 지원 |
| `$system` | 지원 | 지원 |
| `$error`/`$warning`/`$info` | 지원 | 지원 |
| `$fatal` (simulation) | 지원 | 지원 |
| `$fatal` (elaboration) | 부분 지원 | 지원 (issue #1429 fix) |
| `$exit` | 제한적 (program 블록 한계) | 미지원 (`$finish` 사용) |

---

## 합성 가능성

❌ 전 항목 비합성 — 시뮬레이션 및 테스트 인프라 전용.
심각도 태스크(`$fatal`/`$error` 등)는 `/* synthesis translate_off */` 가드 안에 두는 것이 일반적이다.

---

## 본 프로젝트 구현 메모

- **$test$plusargs / $value$plusargs**: 시뮬레이터 초기화 시 커맨드라인을 파싱해
  plusarg 맵(`HashMap<String, Option<String>>`)을 구성.
  `$test$plusargs`는 prefix 매칭으로 키 존재 확인.
  `$value$plusargs`는 format code 파서로 값 변환 후 variable에 저장.
- **$system** ⏳ (미구현): 미배선이라 함수 형태=`E3009` loud-reject, 태스크 형태=`W-ELAB-FEATURE-LIMIT` 경고 후 스킵. 향후 Rust `std::process::Command`로 셸 호출, 반환값=exit status 설계. 시뮬레이션-only 컨텍스트에서만 허용; RTL 내부 호출은 시뮬레이터 에러.
- **$fatal/$error/$warning/$info**: 공통 진단 포매터 + severity level enum.
  `$fatal`은 elaboration 컨텍스트에서 `ElaborationError` raise,
  simulation 컨텍스트에서 `SimFatal` raise.
  `finish_number`는 출력 상세도(verbosity) 설정에 사용.
- **$exit** ✅ (구현): `$finish`의 별칭으로 배선(`"$exit" => SysTaskId::Finish`). `program` 블록은 미지원이라 program-완료-대기 시맨틱은 단순화돼 즉시 `$finish` 동작이다(module-based TB에서 안전). 향후 program block 도입 시 완료 이벤트 채널 구독으로 정밀화 가능.

## Sources

- IEEE 1800-2017 §20.10 ($value$plusargs, $test$plusargs), §20.11 (elaboration severity),
  §20.12 (simulation severity assertion control), §21.3 ($system)
- research-log: [system-tasks-introspection-misc-2026-05-28.md](../../research-log/system-tasks-introspection-misc-2026-05-28.md)
- [chipverify.com — Command Line Input](https://chipverify.com/systemverilog/systemverilog-command-line-input) (WebFetch ✓)
- [theartofverification.com — Plusargs](https://theartofverification.com/plusargs-in-systemverilog/) (WebFetch ✓)
- [accellera.org sv-bc — Severity Tasks](https://www.accellera.org/images/eda/sv-bc/att-5678/severity_tasks_3.htm) (WebFetch ✓)
- [circuitcove.com — Simulation Control Tasks](https://circuitcove.com/system-tasks-simulation-control/) (WebFetch ✓)
- [github.com/verilator — Elaboration tasks issue #1429](https://github.com/verilator/verilator/issues/1429) (WebFetch ✓ — closed/fixed)
