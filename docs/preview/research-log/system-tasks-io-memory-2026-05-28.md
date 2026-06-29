---
title: "SystemVerilog/Verilog System Tasks 조사 — VCD Dump · File I/O · Memory Load"
date: 2026-05-28
author: research-skill (Claude Sonnet 4.6)
scope:
  - IEEE 1364-2005 §18 (VCD dump system tasks)
  - IEEE 1364-2005 §17 / IEEE 1800-2017 §21 (file I/O system tasks)
  - IEEE 1364-2005 §17 / IEEE 1800-2017 §21 (memory load/store tasks)
  - $dumpfile/$dumpvars/$dumpoff/$dumpon/$dumpall/$dumpflush/$dumplimit
  - $fopen/$fclose/$fwrite/$fdisplay/$fmonitor/$fstrobe/$fread/$fscanf/$fgets/$sscanf/$sformat/$sformatf
  - $readmemh/$readmemb/$writememh/$writememb
  - Icarus Verilog (vvp) / Verilator 동작 차이
rounds: 2
status: complete
---

# VCD Dump · File I/O · Memory Load 시스템 태스크 조사

## 배경

Vitamin RTL 시뮬레이터 `hdl-builtins` 크레이트 구현에 앞서,
Phase 1(VCD dump)과 Phase 2(파일·메모리 I/O)에 해당하는 시스템 태스크의
표준 의미론과 주요 시뮬레이터(Icarus · Verilator) 동작 차이를 확인한다.

VCD dump는 이미 [07-vcd-format.md](../hdl-reference/07-vcd-format.md)에서
파일 포맷 명세로 다뤘으나, 이번 조사는 **태스크 호출 시그니처와 시뮬레이터 동작**에 집중한다.

---

## A) VCD Dump 태스크 (IEEE 1364-2005 §18)

### $dumpfile과 $dumpvars — "아무것도 안 한다"의 의미

VCD 파일 생성은 두 단계가 모두 필요하다.
`$dumpfile`은 출력 파일명을 지정하고, `$dumpvars`는 실질적인 덤프를 활성화한다.
`$dumpfile`만 있고 `$dumpvars`가 없으면 VCD 파일이 생성되지 않는다 — no-op.

반대로 `$dumpvars`만 있고 `$dumpfile`이 없으면
기본 파일명("dump.vcd" 또는 구현에 따라 "Verilog.dump")으로 출력된다.

[출처: chipverify.com/verilog/verilog-dump-vcd, WebFetch ✓;
peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00056.htm, WebFetch ✓]

### $dumpvars 인자 변형

가장 중요한 태스크. 인자 조합에 따라 덤프 범위가 달라진다.

| 호출 방식 | 덤프 범위 |
|----------|---------|
| `$dumpvars` | 전체 design — 모든 모듈, 모든 신호 |
| `$dumpvars(0)` | 위와 동일 (level=0 → 무한 깊이) |
| `$dumpvars(1, tb)` | `tb` 모듈의 신호만 (하위 인스턴스 제외) |
| `$dumpvars(0, tb)` | `tb`와 모든 하위 인스턴스 |
| `$dumpvars(2, tb.dut)` | `tb.dut`부터 2 계층 아래까지 |
| `$dumpvars(0, tb.dut.sig_a)` | 개별 신호 `sig_a`만 |
| `$dumpvars(0, tb.ram, tb.alu.a)` | 복수 scope/신호 동시 지정 |

첫 번째 인자가 level, 나머지가 scope 목록이다.
level=0은 "제한 없음(무한 깊이)"를 의미한다.
scope 위치에 개별 신호 이름도 넣을 수 있다.

[출처: chipverify.com/verilog/verilog-dump-vcd WebFetch ✓;
IEEE 1364-2005 §18.2.1]

### $dumpoff / $dumpon

`$dumpoff`를 호출하면 두 가지가 일어난다:

1. 현재 추적 중인 모든 변수에 `x` 값을 VCD에 기록한다
2. 이후 신호 변화를 VCD에 쓰지 않는다 (추적 일시 중단)

`$dumpon`은 덤프를 재개하며, 재개 시점의 현재 신호값을 VCD에 기록한다.
$dumpoff/$dumpon으로 감싼 구간은 파형에서 X로 표시된다.

[출처: peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00056.htm, WebFetch ✓]

### $dumpall

체크포인트(checkpoint)를 삽입한다.
현재 시각의 추적 변수 전체 값을 VCD에 즉시 강제 기록한다.
VCD는 원래 변화가 있을 때만 기록하므로, 전체 상태 스냅샷이 필요할 때 사용한다.

[출처: chipverify.com WebFetch ✓]

### $dumpflush

VCD 내부 버퍼를 파일로 즉시 플러시한다.
시뮬레이터는 I/O 성능을 위해 VCD 데이터를 내부에 버퍼링하는데,
`$dumpflush`는 이 버퍼를 강제 비운다.
장시간 시뮬 중 시뮬레이터 crash 대비, 또는 실시간으로 파형 뷰어를 갱신할 때 유용하다.

[출처: peterfab.com WebFetch ✓; chipverify.com WebFetch ✓]

### $dumplimit

```verilog
$dumplimit(byte_limit);
```

VCD 파일 크기가 `byte_limit` 바이트에 도달하면 덤프를 중단한다.
파일 끝에 `$comment Dump limit reached $end`가 삽입된다.
수십 GB 파일이 되는 것을 방지하기 위한 안전장치.

[출처: chipverify.com WebFetch ✓; 07-vcd-format.md 내부 문서]

### Verilator 동작 — 핵심 주의사항

Verilator는 VCD dump에서 IEEE 표준과 차이가 크다.

| 태스크 | Icarus Verilog | Verilator |
|--------|---------------|-----------|
| `$dumpfile` | 완전 지원 | 지원 |
| `$dumpvars` level | 지원 | **무시** — design top부터 추적 |
| `$dumpvars` scope | 지원 | **무시** — design top부터 추적 |
| `$dumpoff` | 완전 지원 | **currently ignored** |
| `$dumpon` | 완전 지원 | **currently ignored** |
| `$dumpall` | 완전 지원 | **currently ignored** |
| `$dumplimit` | 지원 | **currently ignored** |
| `$dumpflush` | 지원 | 동작 확인 불가 (문서 미언급) |
| 동시 활성 trace | 제한 없음 | 한 번에 하나만 |

Verilator에서 `$dumpvars`의 scope/level 인자는 완전히 무시된다.
Verilator의 VCD 트레이싱은 컴파일 타임 `--trace` 플래그와 코드 레벨 pragma로 제어하는 것이 권장된다.

[출처: verilator.org/guide/latest/languages.html, WebFetch ✓]

---

## B) File I/O 태스크 (IEEE 1800-2017 §21 / IEEE 1364-2005 §17)

### mcd vs fd — 가장 많이 혼동되는 구분

Verilog 파일 I/O에는 두 가지 파일 핸들 방식이 공존한다.

**mcd (multi-channel descriptor)** — Verilog-2001 이전 레거시 방식:
- `$fopen("filename")` 으로 반환 (모드 인자 없음)
- 32비트 비트필드, bit 하나만 set
- bit 0 = stdout (항상 열려있음), bit 31 = 예약
- OR 연산으로 여러 파일에 동시 출력 가능: `$fdisplay(fd1 | fd2, "text")`
- 최대 30개 파일 동시 open

**fd (file descriptor)** — IEEE 1364-2001+ / SV 현대 방식:
- `$fopen("filename", "mode")` 으로 반환 (모드 인자 있음)
- 양의 정수 (C의 FILE* 스타일), 0이면 open 실패
- 읽기 모드, seek, binary I/O 등 고급 기능 사용 가능
- `$fopen("f", "r")` 처럼 모드를 주면 fd 방식

두 방식은 `$fdisplay` 등의 첫 번째 인자로 동일하게 쓸 수 있지만
의미가 다르다 — fd는 특정 파일 하나, mcd는 비트 OR로 여러 파일.

[출처: ovisign.com WebFetch ✓; chipverify.com/verilog/verilog-file-io-operations WebFetch ✓;
hdlworks.com/hdl_corner/verilog_ref/items/SystemFileTasks.htm WebFetch ✓]

### $fopen 모드 전체

| 모드 | 동작 |
|------|------|
| `"r"` / `"rb"` | 읽기 전용 (파일 없으면 실패) |
| `"w"` / `"wb"` | 쓰기 (파일 생성, 기존 내용 삭제) |
| `"a"` / `"ab"` | 추가 쓰기 (파일 없으면 생성) |
| `"r+"` / `"rb+"` | 읽기+쓰기 (기존 파일 필요) |
| `"w+"` / `"wb+"` | 읽기+쓰기 (생성/덮어쓰기) |
| `"a+"` / `"ab+"` | 읽기+추가 (파일 없으면 생성) |

`b` suffix는 binary mode (텍스트 vs 바이너리 줄 끝 처리 차이, Unix에서는 무의미).

[출처: circuitcove.com/system-tasks-file-io/ WebFetch ✓;
hdlworks.com WebFetch ✓]

### 출력 태스크 패밀리 ($fwrite/$fdisplay/$fmonitor/$fstrobe)

```verilog
$fdisplay(fd_or_mcd, "format", args...);  // 개행 자동 추가
$fwrite(fd_or_mcd, "format", args...);    // 개행 없음
$fstrobe(fd_or_mcd, "format", args...);   // Postponed 영역 (NBA 반영)
$fmonitor(fd_or_mcd, "format", args...);  // 인자 변화 시 자동 출력
```

console 출력 태스크($display/$write 등)와 동일하나 첫 인자가 파일 핸들이다.
b/o/h 변형도 동일하게 존재한다: `$fdisplayh`, `$fwriteb` 등.

`$fclose(fd)`는 active `$fmonitor`와 `$fstrobe`를 자동 취소한다.

[출처: chipverify.com/verilog/verilog-file-io-operations WebFetch ✓]

### $fread

```verilog
integer n;
n = $fread(target, fd [, start [, count]]);
```

바이너리 데이터를 읽는다.
`target`이 reg 변수면 해당 폭만큼 읽고, memory 배열이면 순서대로 채운다.
반환값: 실제 읽은 바이트/워드 수 (EOF 또는 오류 시 0 이하).

[출처: hdlworks.com WebFetch ✓]

### $fscanf / $fgets

```verilog
integer n;
n = $fscanf(fd, "format", var1, var2, ...);   // 반환: 매칭 항목 수
n = $fgets(str_var, fd);                       // 반환: 읽은 문자 수 (오류=0)
```

`$fscanf`는 EOF에서 음수(−1)를 반환한다.
`$fgets`는 newline 또는 EOF까지 한 줄을 읽는다.

[출처: hdlworks.com WebFetch ✓; chipverify.com WebFetch ✓]

### $sscanf / $sformat / $sformatf — 문자열 처리

```verilog
// 문자열에서 파싱
integer n;
n = $sscanf(str, "format", var1, var2, ...);

// 문자열로 포맷팅 (task 방식 — void)
$sformat(result_str, "format", args...);

// 문자열로 포맷팅 (function 방식 — string 반환)
string s;
s = $sformatf("format", args...);
```

`$sformat`과 `$sformatf`의 차이:
- `$sformat` — task, 첫 인자 reg에 결과 저장 (void)
- `$sformatf` — function, string 반환값 (SV에서 선호)

[출처: hdlworks.com WebFetch ✓]

### Verilator 파일 I/O 지원

"Generally supported": `$fopen`, `$fclose`, `$fdisplay`, `$ferror`, `$feof`, `$fflush`,
`$fgetc`, `$fgets`, `$fscanf`, `$fwrite`, `$sscanf`

`$fread`, `$sformat`, `$sformatf`는 공식 문서에 명시 없음 — 지원 여부 불확실.

[출처: verilator.org/guide/latest/languages.html, WebFetch ✓]

---

## C) Memory Load/Store 태스크 (IEEE 1364-2005 §17.2)

### $readmemh / $readmemb 시그니처

```verilog
$readmemh("file.hex", mem_array);
$readmemh("file.hex", mem_array, start_addr);
$readmemh("file.hex", mem_array, start_addr, end_addr);

$readmemb("file.bin", mem_array);
$readmemb("file.bin", mem_array, start_addr);
$readmemb("file.bin", mem_array, start_addr, end_addr);
```

`h` = 16진수 데이터, `b` = 2진수 데이터.
`start_addr`, `end_addr`는 선택적으로 로딩 범위를 제한한다.

### 메모리 파일 포맷 규칙 (IEEE 1364-2005 §17.2.8)

| 규칙 | 상세 |
|------|------|
| 데이터 구분자 | 공백(스페이스·탭·개행) |
| 주소 지시자 | `@<hex>` — `@1F`는 다음 데이터를 주소 0x1F부터 로드 |
| 라인 주석 | `//` 이후 줄 끝까지 무시 |
| 블록 주석 | `/* ... */` 내부 전부 무시 |
| 언더스코어 | 숫자 내 `_` 허용 (가독성용, 값에 영향 없음) |
| 4-state 값 | `x`, `z` 포함 가능 (unknown/high-Z) |

### @hex 주소 지시자 사용 예

```text
// rom_init.hex 예시
@00 AA BB CC DD        // 주소 0x00부터 AA, BB, CC, DD 로드
@10 00 11 22 33        // 주소 0x10부터 이어서 로드
// 사이 주소 0x04~0x0F는 변경 없음
```

### 파일 길이와 배열 크기의 불일치

- **파일 < 배열**: `start_addr`부터 데이터가 채워지고, 나머지 배열 원소는 변경 없음 (초기값 유지)
- **파일 > 배열**: 배열 범위를 초과하면 경고 또는 오류 (시뮬레이터 구현 의존)

### $writememh / $writememb

```verilog
$writememh("out.hex", mem_array);
$writememh("out.hex", mem_array, start_addr);
$writememh("out.hex", mem_array, start_addr, end_addr);
```

readmem과 대칭적 포맷으로 파일을 출력한다.
각 주소에 `@<hex_addr>` 형식을 삽입하며 데이터를 기록한다.
이 파일은 `$readmemh`로 다시 읽을 수 있다.

[출처: Verilog::Readmem metacpan.org;
peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00016.htm WebFetch ✓;
ovisign.com WebFetch ✓]

### Verilator 지원

- `$readmemh`, `$readmemb`: 지원. 단 **다차원 배열 미지원** (`reg mem[N][M]` 형태 불가)
- `$writememh`, `$writememb`: 공식 문서에 명시 없음 (지원 불확실)

[출처: verilator.org/guide/latest/languages.html, WebFetch ✓]

---

## Sources

1. **chipverify.com — Verilog VCD Dump** — https://chipverify.com/verilog/verilog-dump-vcd (WebFetch Round 1 ✓)
2. **chipverify.com — Verilog File IO Operations** — https://chipverify.com/verilog/verilog-file-io-operations (WebFetch Round 2 ✓)
3. **chipverify.com — SystemVerilog File IO** — https://www.chipverify.com/systemverilog/systemverilog-file-io (WebFetch Round 2 ✓)
4. **circuitcove.com — File I/O** — https://circuitcove.com/system-tasks-file-io/ (WebFetch Round 1 ✓)
5. **hdlworks.com — System File I/O Tasks** — https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemFileTasks.htm (WebFetch Round 2 ✓)
6. **peterfab.com — Verilog VCD Tasks** — https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00056.htm (WebFetch Round 2 ✓)
7. **peterfab.com — File I/O Functions** — https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00016.htm (WebFetch Round 1 ✓)
8. **projectf.io — Initialize Memory in Verilog** — https://projectf.io/posts/initialize-memory-in-verilog/ (WebFetch Round 1 ✓)
9. **ovisign.com — Verilog Write/Read File Operations** — https://ovisign.com/verilog-verification/verilog-write-read-file-operations/ (WebFetch Round 2 ✓)
10. **verilator.org — Input Languages** — https://verilator.org/guide/latest/languages.html (WebFetch Round 2 ✓)
11. **hdlfactory.com — How to use $readmemh correctly** — https://www.hdlfactory.com/note/2024/10/07/how-to-use-readmemh-correctly/ (WebFetch Round 2 ✓)
12. **IEEE 1364-2005 §18** — VCD dump tasks (primary standard; 직접 fetch 불가, 내용 교차 확인)
13. **IEEE 1800-2017 §21** — File I/O and memory tasks (primary standard; 직접 fetch 불가)
