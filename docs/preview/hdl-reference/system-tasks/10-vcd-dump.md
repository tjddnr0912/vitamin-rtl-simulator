# 10 · VCD Dump System Tasks

## 개요

VCD(Value Change Dump) 파형 파일을 생성·제어하는 태스크 카테고리다.
시뮬레이션 중 신호값 변화를 파일에 기록해 GTKWave 같은 파형 뷰어로 분석할 수 있게 한다.
합성 불가능(simulation-only)이며 `hdl-builtins` dump 카테고리가 구현한다.

VCD 파일 포맷 자체 (헤더·scope·변수 선언·값 변화 형식)는
[07-vcd-format.md](../../07-vcd-format.md)에 별도 정의되어 있다.
이 문서는 태스크 호출 시그니처와 동작에만 집중한다.

## 지원 Phase

- **Phase 1**: `$dumpfile`, `$dumpvars`, `$dumpoff`, `$dumpon`, `$dumpall`, `$dumpflush`, `$dumplimit` **7종 전부 구현**(2026-06-10, format_version 4에서 flush/limit 합류 — limit 도달 시 `$comment Dump limit reached $end` 1회 후 기록 중단)

---

## Phase 1 핵심 원칙

`$dumpfile`과 `$dumpvars`가 둘 다 호출되어야 VCD가 생성된다.
`$dumpfile`만 있고 `$dumpvars`가 없으면 파일이 만들어지지 않는다 — no-op.
`$dumpvars`가 Phase 1의 실질적인 덤프 트리거다.

---

## 항목 상세

### `$dumpfile("filename.vcd")`

- **시그니처**: `$dumpfile(string_filename)`
- **표준**: IEEE 1364-2005 §18.1
- **의미**: VCD를 기록할 파일 경로를 지정한다.
  `$dumpvars` 이전에 호출해야 하며, 생략 시 기본 파일명 `"dump.vcd"`를 사용한다.
  경로는 시뮬레이터 실행 디렉토리 기준 상대 경로 또는 절대 경로 모두 가능하다.
- **반환**: void
- **예시**:

```sv
initial begin
  $dumpfile("tb_output.vcd");
  $dumpvars(0, tb);   // 반드시 $dumpvars까지 호출해야 VCD 생성
end
```

---

### `$dumpvars`

- **시그니처** (세 가지 변형):

  ```sv
  $dumpvars;                                  // 1) 인자 없음
  $dumpvars(level);                           // 2) level만
  $dumpvars(level, scope1 [, scope2, ...]);   // 3) level + scope 목록
  ```

- **표준**: IEEE 1364-2005 §18.2.1
- **의미**: 어떤 신호를 VCD에 기록할지 지정하고 덤프를 시작한다.
  `$dumpvars` 없이는 VCD가 생성되지 않는다.

#### 인자 상세

| 인자 | 타입 | 의미 |
|------|------|------|
| `level` | integer | 계층 탐색 깊이. `0` = 무한 (지정 scope 포함 모든 하위 인스턴스). `1` = 해당 scope만. `2` = 1단 아래까지. |
| `scope` | module/net 참조 | 덤프 대상 모듈 또는 개별 신호. 여러 개 나열 가능. |

#### 호출 변형 예시

```sv
// 1) 인자 없음 — 전체 design 모든 신호
$dumpvars;

// 2) level만 — level 0 = 전체 design (인자 없음과 동일)
$dumpvars(0);

// 3) 특정 scope, 2계층 깊이
$dumpvars(2, tb.dut);

// 4) 특정 scope, 무한 깊이
$dumpvars(0, tb);

// 5) 개별 신호 지정 (scope 위치에 신호명 사용 가능)
$dumpvars(0, tb.dut.sig_valid, tb.dut.data_out);

// 6) 복수 scope 동시 지정
$dumpvars(0, tb.ram_ctrl, tb.alu);
```

- **반환**: void
- **주의**: `$dumpvars` 호출 직후부터 덤프가 시작되는 것이 아니라,
  현재 시뮬 시각이 끝날 때(현재 timestep의 마지막)부터 덤프가 활성화된다.

---

### `$dumpoff`

- **시그니처**: `$dumpoff`
- **표준**: IEEE 1364-2005 §18.3
- **의미**: 덤프를 일시 중단한다. 두 가지 효과가 동시에 발생한다:
  1. 현재 추적 중인 모든 변수에 `x` 값을 VCD에 기록
  2. 이후 신호 변화를 VCD에 쓰지 않음

  파형 뷰어에서 $dumpoff 이후 구간은 모든 신호가 X로 표시된다.
- **반환**: void
- **예시**:

```sv
initial begin
  $dumpfile("sim.vcd");
  $dumpvars(0, tb);
  #1000;
  $dumpoff;      // 초기화 노이즈 구간 제외
  #100;
  $dumpon;       // 실제 테스트 구간만 기록
  #5000;
  $finish;
end
```

---

### `$dumpon`

- **시그니처**: `$dumpon`
- **표준**: IEEE 1364-2005 §18.3
- **의미**: `$dumpoff`로 중단된 덤프를 재개한다.
  재개 시점의 현재 신호값을 VCD에 기록한 후 이후 변화를 계속 추적한다.
- **반환**: void

---

### `$dumpall`

- **시그니처**: `$dumpall`
- **표준**: IEEE 1364-2005 §18.4
- **의미**: 현재 시뮬 시각에서 모든 추적 변수의 현재값을 VCD에 즉시 강제 기록한다.
  VCD는 기본적으로 변화(delta)만 기록하므로, 전체 상태 스냅샷(checkpoint)이
  필요한 시점에 `$dumpall`을 사용한다.
  긴 시뮬레이션에서 중간 상태를 파형 뷰어가 올바르게 렌더링할 수 있도록 돕는다.
- **반환**: void
- **예시**:

```sv
// 1000ns마다 체크포인트 삽입
always #1000 $dumpall;
```

---

### `$dumpflush`

- **시그니처**: `$dumpflush`
- **표준**: IEEE 1364-2005 §18.5
- **의미**: 시뮬레이터 내부 VCD 버퍼를 파일로 즉시 플러시한다.
  시뮬레이터는 I/O 성능을 위해 VCD 데이터를 내부 버퍼에 쌓은 후 일괄 기록하는데,
  `$dumpflush`는 이 버퍼를 즉시 비워 파일에 반영한다.

  사용 시나리오:
  - 장시간 시뮬 중 예상치 못한 종료(crash) 대비
  - 실시간 파형 뷰어(GTKWave live reload)와 함께 사용할 때
- **반환**: void

---

### `$dumplimit(byte_limit)`

- **시그니처**: `$dumplimit(integer byte_limit)`
- **표준**: IEEE 1364-2005 §18.6
- **의미**: VCD 파일 크기 상한을 설정한다.
  파일이 `byte_limit` 바이트에 도달하면 덤프를 자동으로 중단하고
  VCD 파일 끝에 다음을 삽입한다:

  ```vcd
  $comment Dump limit reached $end
  ```

  수 GB 규모가 될 수 있는 VCD를 디스크 공간 초과 없이 제한할 때 사용한다.
- **반환**: void
- **예시**:

```sv
initial begin
  $dumpfile("sim.vcd");
  $dumplimit(100_000_000);   // 100 MB 제한
  $dumpvars(0, tb);
end
```

---

## Icarus / Verilator 동작 차이

| 태스크 | Icarus Verilog | Verilator |
|--------|---------------|-----------|
| `$dumpfile` | 완전 지원 | 지원 |
| `$dumpvars` (인자 없음) | 완전 지원 | 지원 |
| `$dumpvars` level 파라미터 | 완전 지원 | **무시** |
| `$dumpvars` scope 파라미터 | 완전 지원 | **무시** (design top부터) |
| `$dumpoff` | 완전 지원 | **currently ignored** |
| `$dumpon` | 완전 지원 | **currently ignored** |
| `$dumpall` | 완전 지원 | **currently ignored** |
| `$dumplimit` | 지원 | **currently ignored** |
| `$dumpflush` | 지원 | 미명시 |
| 동시 trace 파일 | 제한 없음 | **1개만** 활성 가능 |

**Verilator 권고**: Verilator에서 VCD 트레이싱은 `$dumpvars`의 scope/level 인자를 무시하고
design top 전체를 추적한다. 트레이싱 범위를 제한하려면 컴파일 타임 pragma
(`/* verilator tracing_off */` / `/* verilator tracing_on */`)를 사용해야 한다.

---

## 합성 가능성

❌ 비합성 — 전 태스크가 시뮬레이션 전용.

---

## 본 프로젝트 구현 메모

- `hdl-builtins` 크레이트 `dump` 카테고리가 담당
- `$dumpfile` + `$dumpvars` 쌍이 Phase 1의 핵심 진입점
- `VcdWriter`는 `$dumpvars` 호출 시 활성화되는 수동적 컴포넌트 (07-vcd-format.md 설계 원칙)
- Verilator 호환 모드 구현 시 scope/level 인자 무시 동작을 시뮬레이터 플래그로 제어 예정
- VCD 포맷 상세: [07-vcd-format.md](../../07-vcd-format.md) 참조

## Sources

- IEEE 1364-2005 §18.1–18.6 (VCD dump system tasks — primary standard)
- IEEE 1800-2017 §18 (SystemVerilog, 1364 §18 흡수)
- research-log: [system-tasks-io-memory-2026-05-28.md](../../research-log/system-tasks-io-memory-2026-05-28.md)
- [chipverify.com — Verilog VCD Dump](https://chipverify.com/verilog/verilog-dump-vcd)
- [peterfab.com — Verilog VCD Tasks](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00056.htm)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
- [07-vcd-format.md](../../07-vcd-format.md) (내부 문서, VCD 파일 포맷 명세)
