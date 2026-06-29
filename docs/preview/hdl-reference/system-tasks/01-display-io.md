# 01 · Display · I/O System Tasks

## 개요

시뮬레이션 중 메시지를 stdout에 출력하는 태스크 카테고리다.
printf 스타일 포맷 문자열과 선택적 인자를 받아 콘솔에 텍스트를 쓴다.
합성 불가능(simulation-only)이며 `hdl-builtins` display 카테고리가 구현한다.

## 지원 Phase

- **Phase 1**: `$display`, `$write`, `$monitor`, `$strobe` + b/o/h 변형 16종
- **Phase 2**: `$monitoron` / `$monitoroff` 확장

---

## 항목 상세

### `$display(format_string, arg1, arg2, ...)`

- **시그니처**: `$display([mcd,] "format_string" [, arg1, arg2, ...])`
- **표준**: IEEE 1800-2017 §20.10 / IEEE 1364-2005 §17.1
- **의미**: 호출 즉시 Active/Inactive 이벤트 영역에서 stdout에 출력한다.
  문자열 끝에 **자동 개행**(`\n`)을 붙인다.
  호출 시점 기준의 신호값을 출력하므로 NBA(`<=`) 결과는 반영하지 않는다.
- **반환**: void
- **예시**:

```sv
// 기본 십진 출력
$display("a=%d b=%h time=%t", a, b, $time);

// 자동 개행 확인
$display("first");
$display("second");
// 출력: first\nsecond\n
```

---

### `$displayb`, `$displayo`, `$displayh`

명시적 포맷 specifier가 없는 인자에 적용할 **기본 기수(radix)**를 변경한다.
`b` = 이진, `o` = 8진, `h` = 16진.

```sv
logic [7:0] val = 8'hFF;

$display("val=", val);   // val=255   (기본 십진)
$displayh("val=", val);  // val=ff    (기본 16진)
$displayb("val=", val);  // val=11111111
$displayo("val=", val);  // val=377

// 명시적 specifier는 기본 기수보다 우선
$displayh("val=%d", val); // val=255  (%d가 우선)
```

---

### `$write(format_string, arg1, arg2, ...)`

- **시그니처**: `$write([mcd,] "format_string" [, arg1, arg2, ...])`
- **표준**: IEEE 1800-2017 §20.10 / IEEE 1364-2005 §17.1
- **의미**: `$display`와 동일하나 자동 개행이 **없다**.
  여러 `$write`를 이어서 한 줄에 출력을 조합할 때 사용한다.
- **반환**: void
- **예시**:

```sv
$write("a=%d ", a);
$write("b=%d", b);
$write("\n");          // 명시적 개행 필요
// 출력: a=3 b=7\n
```

---

### `$writeb`, `$writeo`, `$writeh`

`$write` 계열의 기수 변형. `$displayb/o/h`와 동일한 방식으로 기본 기수를 바꾼다.

---

### `$monitor(format_string, arg1, arg2, ...)`

- **시그니처**: `$monitor([mcd,] "format_string" [, arg1, arg2, ...])`
- **표준**: IEEE 1800-2017 §20.12 / IEEE 1364-2005 §17.3
- **의미**: 인자 리스트 내 신호 중 하나라도 값이 바뀔 때마다 자동으로 출력한다.
  출력 시점은 **Postponed 영역** — 현재 시뮬 시각의 모든 이벤트(NBA 포함)가
  완료된 후다. 따라서 NBA 결과가 반영된 최종 값을 볼 수 있다.
  자동 개행 붙음.
- **제약**:
  - 시뮬레이션 전체에서 **하나의 `$monitor`만 활성** 상태를 유지한다.
  - 새 `$monitor` 호출 시 이전 것이 자동 비활성화된다.
- **반환**: void
- **예시**:

```sv
initial $monitor("time=%0t a=%b b=%b", $time, a, b);
// a 또는 b가 바뀔 때마다 자동 출력
```

#### `$monitoron` / `$monitoroff` (Phase 2)

```sv
$monitoroff;   // $monitor 출력 일시 중단
// ... 노이즈가 많은 구간 ...
$monitoron;    // 재개
```

초기화 직후 `$monitoron` 상태가 기본이다.
비활성화 중 발생한 신호 변화는 출력되지 않는다(누락됨, 버퍼링 없음).

---

### `$monitorb`, `$monitoro`, `$monitorh`

`$monitor` 계열의 기수 변형. 명시적 specifier 없는 인자에 기본 기수 적용.

---

### `$strobe(format_string, arg1, arg2, ...)`

- **시그니처**: `$strobe([mcd,] "format_string" [, arg1, arg2, ...])`
- **표준**: IEEE 1800-2017 §20.11 / IEEE 1364-2005 §17.2
- **의미**: **현재 시뮬 시각의 Postponed 영역**에서 출력한다.
  같은 시각에 NBA로 업데이트된 레지스터 최종값을 캡처할 수 있다.
  자동 개행 붙음. `$display`와 달리 지연 출력이므로 NBA 결과가 반영된다.
- **반환**: void
- **예시**:

```sv
always @(posedge clk) begin
  q <= d;                             // NBA: q 업데이트
  $display("q=%b (display)", q);      // NBA 전 q 값 출력
  $strobe("q=%b (strobe)", q);        // NBA 후 q 값 출력 (Postponed)
end
// clk↑에서 d=1이면:
// display: q=0 (NBA 아직 미반영)
// strobe:  q=1 (NBA 반영 후)
```

---

### `$strobeb`, `$strobeo`, `$strobeh`

`$strobe` 계열의 기수 변형.

---

## 포맷 Specifier 상세

IEEE 1800-2017 §20.10 / IEEE 1364-2005 §17.1 기준.

| Specifier | 의미 | 인자 타입 | 비고 |
|-----------|------|----------|------|
| `%d` / `%D` | 십진수 | integer / bit vector | 기본 형식 |
| `%b` / `%B` | 이진수 | bit vector | |
| `%h` / `%H` | 16진수 | bit vector | `%x`/`%X` 동의어 |
| `%x` / `%X` | 16진수 | bit vector | `%h`와 동일 |
| `%o` / `%O` | 8진수 | bit vector | |
| `%c` / `%C` | ASCII 문자 | 8-bit | 하위 8비트 |
| `%s` / `%S` | 문자열 | string / byte array | |
| `%t` / `%T` | 시간 | time | IEEE: `$timeformat` 영향. **vitamin: plain decimal(`%0d` 동치), `$timeformat`·기본 필드폭 미적용** |
| `%v` / `%V` | net 신호 강도 | net (4-state) | strength + value |
| `%e` / `%E` | 실수 지수 표기 | real | 예: `1.23e+02` |
| `%f` / `%F` | 실수 소수 표기 | real | 예: `123.000000` |
| `%g` / `%G` | 실수 자동 선택 | real | e/f 중 짧은 쪽 |
| `%m` / `%M` | 계층 모듈 이름 삽입 | (인자 불필요) | 디버그용 |
| `%p` / `%P` | assignment pattern | struct/enum/dynamic | SV §20.10.2 |
| `%u` | 비형식 2-value 데이터 | bit vector | 이진 덤프 |
| `%z` | 비형식 4-value 데이터 | bit vector | 4-state 덤프 |
| `%l` / `%L` | 라이브러리 바인딩 이름 | (인자 불필요) | |

### 폭 수정자

| 예시 | 의미 |
|------|------|
| `%6d` | 필드 폭 6, 오른쪽 정렬 (leading space) |
| `%06d` | 필드 폭 6, 0 패딩 |
| `%0d` / `%0h` | 최소 폭 (leading space/zero 없음) |
| `%6.2f` | 실수 전체 폭 6, 소수점 이하 2자리 |

```sv
// 폭 수정자 예시
$display("%6d", 42);    // "    42"  (4 spaces + 42)
$display("%06d", 42);   // "000042"
$display("%0d", 42);    // "42"      (compact)
$display("%0h", 8'hA);  // "a"       (compact hex)
```

---

## Icarus / Verilator 동작 차이

| 항목 | Icarus Verilog | Verilator |
|------|---------------|-----------|
| Active 영역 $display | 표준 준수 | 표준 준수 |
| combo 블록 $display | 1회 실행 | 복수 실행 가능 (이벤트 재정렬) |
| $strobe | 완전 지원 | 지원 |
| $monitor | 완전 지원 | 지원 |
| $monitoron/off | 지원 | 지원 |
| 4-state %v specifier | 지원 | Z→0 처리 (2-state 한계) |

**Verilator 권고**: `always_comb` / `always @(*)` 블록 내 `$display` 사용을 피할 것.
같은 시뮬 시각에 combo 블록이 여러 번 재평가되면 `$display`도 중복 실행된다.
sequential 블록(`always_ff`, `initial`) 또는 `$strobe` 사용을 권장한다.

---

## 합성 가능성

❌ 비합성 — 전 태스크가 시뮬레이션 전용.
합성 도구는 `$display` 계열 호출을 무시한다.

---

## 본 프로젝트 구현 메모

- `hdl-builtins` 크레이트 `display` 카테고리가 담당
- b/o/h 변형 16종 모두 Phase 1에서 구현 (기수 파라미터로 통합 처리)
- MCD (multi-channel descriptor) 파라미터는 Phase 2 file-io와 연동 후 지원

## Sources

- IEEE 1800-2017 §20.10 (display/write), §20.11 (strobe), §20.12 (monitor)
- IEEE 1364-2005 §17.1, §17.2, §17.3
- research-log: [system-tasks-display-time-2026-05-28.md](../../research-log/system-tasks-display-time-2026-05-28.md)
- [hdlworks.com System Display Tasks](https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemDisplayTasks.htm)
- [chipverify.com Verilog Display Tasks](https://chipverify.com/verilog/verilog-display-tasks)
- [peterfab.com Verilog Display Tasks](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00013.htm)
- [circuitcove.com Format Specifiers](https://circuitcove.com/system-tasks-format-spec/)
