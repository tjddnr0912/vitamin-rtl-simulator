# 08 · Verilog 게이트 수준 모델링 (Gate-Level Modeling)

IEEE 1364-2001/2005 기준. 모듈 선언 없이 즉시 인스턴스화할 수 있는 내장 프리미티브
28종과, 사용자가 직접 정의하는 UDP(User-Defined Primitive)를 다룬다.

---

## 내장 게이트 프리미티브 분류표

| 분류 | 프리미티브 종류 | 포트 구조 | 개수 |
|------|--------------|----------|------|
| 논리 게이트 (다중 입력) | `and` `or` `nand` `nor` `xor` `xnor` | 출력 1 + 입력 N | 6 |
| 버퍼 / 인버터 (단일 입력) | `buf` `not` | 출력 N + 입력 1 | 2 |
| 삼상 버퍼 | `bufif0` `bufif1` `notif0` `notif1` | 출력 1 + 입력 1 + 제어 1 | 4 |
| MOS 스위치 (단방향) | `nmos` `pmos` `rnmos` `rpmos` | 출력 + 데이터 + 제어 | 4 |
| CMOS 스위치 (단방향) | `cmos` `rcmos` | 출력 + 데이터 + n제어 + p제어 | 2 |
| 양방향 스위치 | `tran` `rtran` `tranif0` `tranif1` `rtranif0` `rtranif1` | inout × 2 (+ 제어) | 6 |
| Pull 소스 | `pullup` `pulldown` | net 1개 이상 | 2 |
| **합계** | | | **26종** (tran 계열 6종 포함) |

> `xor` / `xnor`는 이론상 입력 2개에서 시작하지만 대부분의 합성 도구가 정확히 2입력만
> 지원한다. 안전하게 2입력으로 제한한다.

---

## 논리 게이트

출력이 첫 번째 포트, 이후 모든 포트가 입력이다. 입력 개수 제한 없음.

```verilog
// 기본 인스턴스화
and  g1  (y, a, b);             // 2입력 AND
or   g2  (y, a, b, c);          // 3입력 OR
nand g3  (y, a, b);             // NAND
nor       (y, a, b);            // 인스턴스 이름 생략 가능
xor  g5  (y, a, b);             // XOR (2입력 권장)
xnor g6  (y, a, b);             // XNOR

// 배열 인스턴스화 — 4비트 버스에 동일 게이트 4개
and [3:0] ga (y_bus, a_bus, b_bus);
```

진리표:

| a | b | and | nand | or | nor | xor | xnor |
|---|---|-----|------|-----|-----|-----|------|
| 0 | 0 | 0 | 1 | 0 | 1 | 0 | 1 |
| 0 | 1 | 0 | 1 | 1 | 0 | 1 | 0 |
| 1 | 0 | 0 | 1 | 1 | 0 | 1 | 0 |
| 1 | 1 | 1 | 0 | 1 | 0 | 0 | 1 |

입력에 `x`가 있으면 출력도 `x`가 될 수 있다 (결정 불가능한 경우).

---

## 버퍼와 인버터

`buf`는 출력을 여러 개 가질 수 있다 (팬아웃). 입력은 반드시 하나:

```verilog
buf  b1 (y1, y2, y3, in);   // 출력 3개, 입력 1개 (마지막이 입력)
not  n1 (y, in);
```

---

## 삼상 버퍼 (Tri-state Buffer)

제어 신호(ctrl)가 활성이면 출력이 구동되고, 비활성이면 고임피던스(Z)가 된다.

| 프리미티브 | 활성 ctrl | 비활성 ctrl | 비고 |
|-----------|---------|-----------|------|
| `bufif1` | ctrl = 1 → `out = in` | ctrl = 0 → `out = Z` | 정극성 |
| `bufif0` | ctrl = 0 → `out = in` | ctrl = 1 → `out = Z` | 역극성 |
| `notif1` | ctrl = 1 → `out = ~in` | ctrl = 0 → `out = Z` | 반전 + 정극성 |
| `notif0` | ctrl = 0 → `out = ~in` | ctrl = 1 → `out = Z` | 반전 + 역극성 |

```verilog
bufif1 tb1 (out, in, ctrl);    // ctrl=1이면 버퍼, ctrl=0이면 Z
bufif0 tb2 (out, in, oe_n);    // oe_n=0(활성 LOW)이면 버퍼
notif1 ti1 (out, in, ctrl);    // ctrl=1이면 인버터, ctrl=0이면 Z
```

---

## MOS 스위치 (단방향)

아날로그 CMOS 수준 시뮬레이션용. RTL 합성 대상이 아니다.

### nmos / pmos / rnmos / rpmos — 3포트

```
(출력, 입력_데이터, 제어)
```

| 프리미티브 | 도통 조건 | 저항성 |
|-----------|---------|--------|
| `nmos` | 제어 = 1 | ❌ |
| `pmos` | 제어 = 0 | ❌ |
| `rnmos` | 제어 = 1 | ✅ (약한 신호 전달) |
| `rpmos` | 제어 = 0 | ✅ |

```verilog
nmos  nm1 (out, data, ctrl);    // ctrl=1이면 data → out
pmos  pm1 (out, data, ctrl);    // ctrl=0이면 data → out
rnmos rnm (out, data, ctrl);    // 저항성 nmos (strength 감쇠)
rpmos rpm (out, data, ctrl);    // 저항성 pmos
```

### cmos / rcmos — 4포트

```
(출력, 입력_데이터, n제어, p제어)
```

nmos와 pmos를 하나로 묶은 구조. n제어=1, p제어=0일 때 도통:

```verilog
cmos  cm1 (out, data, nctrl, pctrl);
rcmos rcm (out, data, nctrl, pctrl);
```

---

## 양방향 스위치

두 포트가 모두 inout이다. 신호가 양방향으로 흐른다.
**drive strength 지정 불가. 합성 지원 없음.**

| 프리미티브 | 제어 | 도통 조건 | 저항성 |
|-----------|------|---------|--------|
| `tran` | 없음 | 항상 도통 | ❌ |
| `rtran` | 없음 | 항상 도통 | ✅ |
| `tranif1` | ctrl | ctrl = 1 | ❌ |
| `tranif0` | ctrl | ctrl = 0 | ❌ |
| `rtranif1` | ctrl | ctrl = 1 | ✅ |
| `rtranif0` | ctrl | ctrl = 0 | ✅ |

```verilog
tran     tr1 (inout1, inout2);              // 항상 도통 (패스게이트 단순 연결)
rtran    rtr (inout1, inout2);              // 저항성 항상 도통
tranif1  ti1 (inout1, inout2, ctrl);        // ctrl=1이면 도통
tranif0  ti0 (inout1, inout2, ctrl);        // ctrl=0이면 도통
rtranif1 ri1 (inout1, inout2, ctrl);
rtranif0 ri0 (inout1, inout2, ctrl);
```

---

## Pull 소스

net을 약하게 풀업/풀다운한다. 다른 드라이버가 없을 때 기본값을 설정하는 용도:

```verilog
pullup  pu1 (net_a);        // net_a → logic 1 (pull 강도)
pulldown pd1 (net_b);       // net_b → logic 0 (pull 강도)
pullup  (pull1) pu2 (sda);  // 강도 명시
```

---

## Drive Strength

논리 게이트와 UDP에 적용 가능하다. MOS 스위치와 tran 계열에는 사용 불가.

### 강도 레벨 (낮은 것부터 높은 것 순서)

| 레벨 | 이름 | 키워드 (1 강도) | 키워드 (0 강도) | 적용 대상 |
|------|------|--------------|--------------|----------|
| 0 | High-Z | `highz1` | `highz0` | 게이트/net |
| 1 | Small | `small` | — | trireg 전용 |
| 2 | Medium | `medium` | — | trireg 전용 |
| 3 | Weak | `weak1` | `weak0` | 게이트/net |
| 4 | Large | `large` | — | trireg 전용 |
| 5 | Pull | `pull1` | `pull0` | 게이트/net |
| 6 | Strong | `strong1` | `strong0` | 게이트/net (기본값) |
| 7 | Supply | `supply1` | `supply0` | 게이트/net |

기본값은 `(strong1, strong0)`. `(highz0, highz1)` 또는 `(highz1, highz0)` 조합은
불법이다.

### 적용 구문

```verilog
// gate (strength1, strength0) #(delay) instance_name (ports);
and  (strong1, weak0)   g1 (out, a, b);
or   (supply1, pull0)   g2 (out, a, b);
buf  (weak1,   weak0)   b1 (out, in);

// assign에도 적용
assign (weak1, strong0) net_q = data;
```

### 합성 주의사항

drive strength는 **시뮬레이션 전용** 개념이다. 합성 도구 대부분은 `supply` / `weak`
강도 지정을 무시하거나 경고를 낸다. RTL 논리를 drive strength에 의존하면
pre/post-synthesis 시뮬레이션 불일치가 발생한다.

---

## UDP (User-Defined Primitive)

`primitive...endprimitive` 블록으로 정의. 모듈과 동급 레벨에 위치한다(모듈 안이 아님).

**공통 제약**:
- 출력 포트 반드시 1개, 모든 포트는 1비트 스칼라
- Z 값 출력 불가 (출력은 0 / 1 / x만)
- 모듈처럼 인스턴스화해 사용

### 조합형 UDP

출력이 입력의 논리 조합만으로 결정된다. 입력 최대 10개.
테이블의 각 행은 `입력들 : 출력;` 형식:

```verilog
// 2-to-1 MUX UDP
primitive mux2 (out, sel, a, b);
    output out;
    input  sel, a, b;
    table
        //  sel  a  b  :  out
            0    0  ?  :  0;     // sel=0, a=0 → out=0 (?는 b 무관)
            0    1  ?  :  1;     // sel=0, a=1 → out=1
            1    ?  0  :  0;     // sel=1, b=0 → out=0
            1    ?  1  :  1;     // sel=1, b=1 → out=1
            x    0  0  :  0;     // sel=x이지만 a=b=0 → 결정 가능
            x    1  1  :  1;     // sel=x이지만 a=b=1 → 결정 가능
    endtable
endprimitive
```

### 순차형 UDP

출력이 `reg`로 선언된다. 입력 최대 9개. 테이블은 `입력들 : 현재상태 : 다음상태;` 형식.

**레벨 감지형 (래치)**:

```verilog
// D 래치 UDP
primitive dlatch (q, clk, d);
    output reg q;
    input  clk, d;
    initial q = 0;              // 초기값 (선택적)
    table
        //  clk  d  :  q(현재)  :  q(다음)
            1    0  :  ?        :  0;     // clk=1이면 d=0 → q=0
            1    1  :  ?        :  1;     // clk=1이면 d=1 → q=1
            0    ?  :  ?        :  -;     // clk=0이면 유지 (- = no change)
    endtable
endprimitive
```

**에지 감지형 (플립플롭)**:

```verilog
// 상승 에지 D 플립플롭 UDP
primitive dff (q, clk, d);
    output reg q;
    input  clk, d;
    initial q = 0;
    table
        //  clk  d  :  q  :  q_next
            r    0  :  ?  :  0;     // 상승 에지 + d=0
            r    1  :  ?  :  1;     // 상승 에지 + d=1
            f    ?  :  ?  :  -;     // 하강 에지 — 변화 없음
            ?    *  :  ?  :  -;     // 클록 외 입력 변화 — 변화 없음
    endtable
endprimitive
```

### 테이블 심볼 정리

| 심볼 | 의미 | 위치 |
|------|------|------|
| `0` | logic 0 | 입력 / 현재상태 / 다음상태 |
| `1` | logic 1 | 입력 / 현재상태 / 다음상태 |
| `x` | 불확정 | 입력 / 현재상태 / 다음상태 |
| `?` | 0 · 1 · x 중 어느 것 | 입력 / 현재상태만 |
| `b` | 0 또는 1 | 입력만 |
| `*` | 어떤 변화든 (`??`) | 입력만 |
| `-` | 출력 변화 없음 | 다음상태만 |
| `r` | 상승 에지 (0→1) | 입력만 (에지 감지) |
| `f` | 하강 에지 (1→0) | 입력만 (에지 감지) |
| `p` | 잠재 상승 (01, 0x, x1) | 입력만 |
| `n` | 잠재 하강 (10, 1x, x0) | 입력만 |

### UDP 인스턴스화

```verilog
module top;
    wire out_mux, out_q;
    wire sel, a, b, clk, d;

    mux2  u_mux (.out(out_mux), .sel(sel), .a(a), .b(b));
    dff   u_ff  (.q(out_q), .clk(clk), .d(d));
endmodule
```

모듈과 동일한 방식으로 named/positional 포트 연결이 가능하다.

---

## 게이트 수준 지연 (Delay)

모든 게이트와 UDP에 delay를 지정할 수 있다:

```verilog
and #(2)         g1 (y, a, b);         // 모든 전이에 2 time-unit
and #(2, 3)      g2 (y, a, b);         // rise=2, fall=3
and #(2, 3, 4)   g3 (y, a, b);         // rise=2, fall=3, turn-off=4
and #(1:2:3)     g4 (y, a, b);         // min:typ:max 단일 지정
and #(1:2:3, 1:3:5) g5 (y, a, b);      // rise min:typ:max, fall min:typ:max
```

---

## Sources

- IEEE 1364-2001 §7 (gate and switch level modeling), §7.6 (UDP)
- IEEE 1800-2017 §28 (gate and switch level modeling)
- vlsiverify.com/verilog/gate-level-modeling/ (WebFetch 검증 ✓)
- vlsiverify.com/verilog/user-defined-primitives/ (WebFetch 검증 ✓, 심볼 목록)
- vlsiverify.com/verilog/strength-in-verilog/ (WebFetch 검증 ✓, drive strength 8레벨)
- peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00003.htm (WebFetch 검증 ✓, 프리미티브 전체 목록)
- chipverify.com/verilog/verilog-gate-level-modeling (gate 분류 및 진리표)
- chipverify.com/verilog/verilog-udp (UDP 구조 WebFetch 검증 ✓)
