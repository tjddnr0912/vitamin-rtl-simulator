# 03 · SystemVerilog 절차 구문

IEEE 1800-2017 §9/§12/§13 기준. SV는 Verilog의 `always`/`initial`에서 설계 의도를 명시하는
세 가지 변형(`always_comb/ff/latch`)을 추가하고, case/if 수식어, 루프 확장, 함수/태스크
인자 전달 방식을 개선했다.

---

## always_comb / always_ff / always_latch

Verilog의 `always @(*)` 하나로 조합·순차·래치를 모두 표현했던 것을 SV에서 분리했다.
툴이 의도와 실제 코드가 불일치하면 경고 또는 에러를 발생시킨다.

### always_comb (§9.2.2.2)

조합 논리(combinational logic) 전용.

```systemverilog
always_comb begin
    y = a & b;
    case (sel)
        2'b00: out = x;
        2'b01: out = y;
        default: out = '0;
    endcase
end
```

**의미론**:
- 묵시적 sensitivity list: 블록 내에서 **읽히는** 모든 신호 자동 포함. LHS 피대입 신호는 제외.
- 시간 0에 자동 1회 실행 (`always @*`는 첫 변화 이벤트를 기다린다).
- 동일 신호에 대한 단일 드라이버를 컴파일 타임에 강제 — 다른 `always`나 `assign`이
  같은 변수에 쓰면 컴파일 에러.
- 블로킹 타이밍 제어(`#delay`, 이벤트 `@`) 사용 불가.

**주의**: 블록 내에서 읽는 함수의 내부 신호는 sensitivity list에 포함되지 않는다.
함수 내부 신호 변화가 블록을 재트리거하지 않는 동작에 주의.

### always_ff (§9.2.2.4)

클록 엣지 기반 레지스터(플립플롭) 전용.

```systemverilog
always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) q <= '0;
    else        q <= d;
end
```

**의미론**:
- 정확히 하나의 이벤트 제어 `@(...)` 만 허용. 블록 내 추가 이벤트/딜레이 금지.
- 비블로킹 대입(`<=`) 사용 권장 (blocking 대입 사용 시 툴 경고).
- 합성 툴이 플립플롭으로 추론.

### always_latch (§9.2.2.3)

레벨 감지 래치(latch) 전용.

```systemverilog
always_latch begin
    if (en) q <= d;   // en=1일 때만 투명, en=0이면 값 유지
end
```

**의미론**:
- `always_comb`와 동일한 묵시적 sensitivity 규칙.
- 불완전한 조건 분기(래치 동작)를 의도한 것으로 간주 — 합성 툴의 래치 경고를 억제.
- 의도하지 않은 래치가 있으면 `always_comb`나 `always_ff`를 쓰는 것이 맞다.

---

## unique / priority case·if

Verilog의 `case`/`if`는 시뮬레이션과 합성 사이에 해석 차이가 있었다. SV에서 두 수식어로
의도를 명시하면 런타임 검사와 합성 힌트를 동시에 얻는다.

### unique case / unique if

```systemverilog
unique case (opcode)
    4'hA: y = a + b;
    4'hB: y = a - b;
    4'hC: y = a & b;
    default: y = '0;
endcase
```

**런타임 검사**: 정확히 하나의 분기만 일치해야 한다.
- 어떤 분기도 일치하지 않으면 경고 (default 없는 경우).
- 두 개 이상의 분기가 동시에 일치하면 경고.

**합성 힌트**: 모든 분기가 상호 배타적 — 툴이 우선순위 인코더 없이 단순화 가능.

### priority case / priority if

```systemverilog
priority casez (addr)
    8'b1???_????: region = BOOT;
    8'b01??_????: region = ROM;
    default:      region = RAM;
endcase
```

**런타임 검사**: 첫 번째 일치 분기를 실행. 어떤 분기도 일치하지 않으면 경고.

**합성 힌트**: 위에서 아래로 우선순위가 있음 — 툴이 우선순위 인코더를 추론.

`unique0`/`priority0` 변형은 불일치 시 경고를 억제한다 (SV §12.4.2).

---

## foreach

배열 전체를 순회하는 전용 루프. 다차원 배열은 인덱스를 쉼표로 나열한다.

```systemverilog
// 1차원
int arr [8];
foreach (arr[i])
    arr[i] = i * 2;

// 다차원
int mat [4][4];
foreach (mat[r, c])
    mat[r][c] = r * 4 + c;

// dynamic array
int dyn[];
dyn = new[5];
foreach (dyn[k])
    dyn[k] = k;

// associative array
int aa[string];
foreach (aa[k])
    $display("%s = %0d", k, aa[k]);
```

---

## do-while

최소 한 번 실행을 보장하는 루프.

```systemverilog
int i = 0;
do begin
    $display("i = %0d", i);
    i++;
end while (i < 5);
```

Verilog의 `while`은 조건을 먼저 검사하므로 초기 조건이 거짓이면 한 번도 실행되지 않는다.
`do-while`은 루프 본체가 적어도 한 번은 필요한 경우에 적합하다.

---

## void function + return

반환값이 없는 함수. Verilog의 task와 달리 타이밍 제어가 불가하며, `return`으로 중간 탈출 가능.

```systemverilog
function void check_range(int val, int lo, int hi);
    if (val < lo || val > hi) begin
        $display("out of range: %0d", val);
        return;           // 여기서 함수 탈출
    end
    $display("ok: %0d", val);
endfunction
```

- 반환값이 필요하면 `function int ...`처럼 반환 타입을 지정.
- `return expr;` 형태로 값을 반환. `void function`에서 `return expr;`은 컴파일 에러.

---

## ref / const ref 인자

Verilog의 함수/태스크는 값 복사(pass by value)만 지원했다. SV는 참조 전달을 추가했다.

### ref — 참조 전달

원본 변수를 직접 수정한다. 대용량 배열 전달 시 복사 비용이 0이다.
`automatic` 함수/태스크에서만 사용 가능.

```systemverilog
function automatic void zero_out(ref int arr[]);
    foreach (arr[i]) arr[i] = 0;
endfunction

int data[] = new[1024];
zero_out(data);   // data 원본을 직접 0으로 초기화
```

### const ref — 읽기 전용 참조

참조로 전달되지만 수정 불가. 쓰기 시도 시 컴파일 에러.
읽기 전용 대용량 인자를 안전하게 전달할 때 사용한다.

```systemverilog
function automatic int sum_all(const ref int arr[]);
    int total = 0;
    foreach (arr[i]) total += arr[i];
    // arr[0] = 0;   // 컴파일 에러
    return total;
endfunction
```

### 비교

| 전달 방식 | 키워드 | 원본 수정 | 복사 비용 | 용도 |
|----------|--------|----------|----------|------|
| 값 복사 | (없음) | 불가 | O(n) | 소형 스칼라 |
| 참조 | `ref` | 가능 | O(1) | 대용량 배열, 다중 출력 |
| 읽기 전용 참조 | `const ref` | 불가 | O(1) | 대용량 읽기 전용 인자 |

---

## Sources

- IEEE 1800-2017 §9 (Processes), §12 (Procedural programming statements), §13 (Tasks and functions)
- verilogpro.com/systemverilog-always_comb-always_ff/
- verilogpro.com/systemverilog-unique-priority/
- vlsiworlds.com/system-verilog/argument-passing-and-the-const-keyword-in-systemverilog/
- chipverify.com/systemverilog/systemverilog-functions
- verificationguide.com/systemverilog/systemverilog-task-function-argument-passing/
