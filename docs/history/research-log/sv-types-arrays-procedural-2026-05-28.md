# SystemVerilog 데이터 타입·배열·절차 구문 조사
**대상**: IEEE 1800-2017 §6/§7/§9/§12/§13
**날짜**: 2026-05-28
**조사 방식**: WebSearch 3라운드 + WebFetch 5회 1차 검증

---

## 조사 동기

SystemVerilog는 Verilog(IEEE 1364-2005)를 완전히 흡수하면서 세 가지 큰 확장을 추가했다.
(A) 데이터 타입: 2-state 타입 추가 및 `logic`으로 net/variable 통합,
(B) 배열: 동적·연상·큐 등 4종 추가 및 메소드 라이브러리,
(C) 절차 구문: `always_comb/ff/latch`, `unique/priority`, `foreach`, `void function`, `ref` 등.
본 조사는 각 항목의 의미론, 제약, 합성 가능 여부를 1차 자료까지 검증하여 정리한다.

---

## (A) 데이터 타입

### 2-state vs 4-state 구조

Verilog의 데이터 타입은 모두 4-state(0/1/X/Z)였다. SV는 X/Z가 없는 2-state 타입을
별도로 도입해 시뮬레이션 속도를 높이고 소프트웨어 정수 연산과의 호환성을 개선했다.

**4-state 타입 (Verilog 계승)**

| 타입 | 크기 | 부호 | 용도 |
|------|------|------|------|
| `logic` | 가변 | unsigned | SV 신규. net+variable 통합 대체. 단일 드라이버 컴파일 검사 |
| `reg` | 가변 | unsigned | 레거시. `logic`으로 마이그레이션 권장 |
| `integer` | 32비트 | signed | 레거시. `int`으로 마이그레이션 권장 |
| `time` | 64비트 | unsigned | `$time` 저장용, 합성 불가 |

**2-state 타입 (SV 신규)**

| 타입 | 크기 | 부호 | 비고 |
|------|------|------|------|
| `bit` | 가변 | unsigned | 가변 폭 2-state, `logic`의 2-state 버전 |
| `byte` | 8비트 | signed | ASCII 문자 또는 정수 |
| `shortint` | 16비트 | signed | — |
| `int` | 32비트 | signed | C `int`와 동등 |
| `longint` | 64비트 | signed | C `long long`과 동등 |

2-state 타입은 초기화되지 않아도 기본값이 0(4-state는 X). 시뮬레이션 속도 이점은 있으나
RTL에서 X-propagation이 필요한 경우에는 `logic`을 유지해야 한다.

### logic vs reg 구분

`logic`은 SV §6.3.4에서 정의된 4-state 변수 타입이다. Verilog에서 `wire`와 `reg`를 쓸 때
어느 컨텍스트에서 대입할 수 있는지를 암기해야 했던 부담을 없앤다. 컴파일러가 동일 신호에 대해
복수의 드라이버를 감지하면 에러를 발생시킨다(단일 드라이버 강제). 다중 드라이버 버스
(`tri`, `wor` 등)는 여전히 기존 net 타입을 사용해야 한다.

### Enum

```systemverilog
// 명시 값 — 지정하지 않은 항목은 이전+1 자동 증가
typedef enum logic [1:0] {
    IDLE  = 2'b00,
    RUN   = 2'b01,
    DONE  = 2'b10,
    ERROR             // 자동 = 2'b11
} state_e;
```

내장 메소드 (IEEE 1800-2017 §6.19):

| 메소드 | 동작 |
|--------|------|
| `.first()` | 첫 번째 멤버 값 반환 |
| `.last()` | 마지막 멤버 값 반환 |
| `.next(N)` | N번째 다음 값 (기본 N=1) |
| `.prev(N)` | N번째 이전 값 (기본 N=1) |
| `.num()` | 멤버 총 개수 반환 |
| `.name()` | 현재 값의 문자열 표현 반환 |

### Struct — packed vs unpacked

**packed struct**: 멤버들이 연속 비트 벡터로 매핑. 슬라이싱 가능. 합성 가능.

```systemverilog
typedef struct packed {
    logic [3:0]  opcode;
    logic [11:0] address;
    logic [7:0]  data;
} instr_t;   // 24비트 단일 벡터
```

**unpacked struct**: 멤버 사이 갭 허용. 합성 불가. 검증/모델링용.

```systemverilog
typedef struct {
    int     id;
    string  name;
    real    score;
} student_t;
```

`rand`/`randc` 한정자를 필드에 붙이면 클래스 무작위화(`randomize()`)에 포함된다.

### Union — packed vs tagged

**packed union**: 모든 멤버가 동일 비트 폭이어야 함. 합성 가능. 타입 안전 없음.

```systemverilog
typedef union packed {
    logic [31:0]       word;
    logic [3:0][7:0]   bytes;
} word_u;
```

**tagged union** (SV §7.3.2): 마지막에 쓴 멤버를 내부 태그로 기록. 다른 멤버로 읽으면
런타임 에러. 멤버 크기가 달라도 됨. 합성 불가.

```systemverilog
typedef union tagged {
    int        a;
    byte       b;
    bit [15:0] c;
} data_t;
data_t d;
d = tagged a 32'hffff;
// d.b;  → 런타임: "Invalid member usage of a tagged union"
```

### 기타 타입

- **typedef**: 타입 별칭 선언. 주로 struct/union/enum에 이름을 부여.
- **string**: 동적 크기 문자열. `.len()`, `.toupper()`, `.atoi()` 등 메소드. 합성 불가.
- **chandle**: DPI를 통해 C/C++ 포인터를 전달하는 불투명 핸들 타입.
- **virtual interface**: interface 인스턴스에 대한 핸들. 클래스는 포트를 가질 수 없어
  신호 접근이 불가능하기 때문에, virtual interface가 클래스와 하드웨어 신호를 연결하는 교각.

---

## (B) 배열

### 5가지 배열 종류

| 종류 | 선언 | 크기 결정 | 합성 |
|------|------|----------|------|
| Packed | `logic [3:0][7:0] m` | 컴파일타임 | 가능 |
| Unpacked | `int arr [0:7]` | 컴파일타임 | 가능(단순) |
| Dynamic | `int d[]` | 런타임 `new[N]` | 불가 |
| Associative | `int aa[string]` | 자동 해시 | 불가 |
| Queue | `int q[$]` | 런타임 push/pop | 불가 |

### Dynamic Array

```systemverilog
int dyn[];
dyn = new[8];            // 8개 할당
dyn = new[16](dyn);      // 16개로 재할당, 기존 내용 복사
dyn.delete();            // 전체 해제 (size → 0)
```

### Associative Array

```systemverilog
int aa[string];
aa["alpha"] = 1;
if (aa.exists("alpha")) ...    // 키 존재 확인 → 1/0 반환
aa.delete("alpha");            // 특정 키 삭제
aa.delete();                   // 전체 삭제
int n = aa.num();              // 엔트리 수

string k;
aa.first(k);   // 첫 번째 키 → k에 저장. 비어있으면 0 반환
aa.last(k);    // 마지막 키
aa.next(k);    // k 다음 키를 k에 덮어씀. 마지막이거나 비어있으면 0 반환
aa.prev(k);    // k 이전 키를 k에 덮어씀
```

와일드카드 인덱스 `[*]`: 임의 정수 표현식을 키로 사용.

### Queue

```systemverilog
int q[$];
q.push_back(1);           // 뒤에 추가
q.push_front(0);          // 앞에 추가
int v = q.pop_back();     // 뒤 요소 꺼냄 + 제거
int v = q.pop_front();    // 앞 요소 꺼냄 + 제거
q.insert(2, 99);          // 인덱스 2에 삽입
q.delete(2);              // 인덱스 2 삭제
q.delete();               // 전체 삭제
int s = q.size();         // 현재 크기
```

### 배열 메소드 라이브러리

**정렬·재배열** (dynamic/unpacked 배열 적용, 배열 원소 직접 수정):

| 메소드 | 동작 | with 절 |
|--------|------|---------|
| `.sort()` | 오름차순 정렬 | 선택 (키 표현식) |
| `.rsort()` | 내림차순 정렬 | 선택 |
| `.reverse()` | 순서 역전 (원소 수정 없음) | 불가 |
| `.shuffle()` | 무작위 섞기 | 불가 |

**위치 탐색** (반환값: 큐):

| 메소드 | 반환 |
|--------|------|
| `.find with (item > 3)` | 조건 만족 요소 큐 |
| `.find_first with (...)` | 첫 번째 요소 큐 |
| `.find_last with (...)` | 마지막 요소 큐 |
| `.find_index with (...)` | 조건 만족 인덱스 큐 |
| `.find_first_index with (...)` | 첫 번째 인덱스 큐 |

조건 불일치 시 빈 큐 반환.

**축소 메소드** (스칼라 반환, with 절로 요소 변환 가능):

| 메소드 | 동작 |
|--------|------|
| `.sum()` | 전체 합 |
| `.product()` | 전체 곱 |
| `.and()` | 비트 AND |
| `.or()` | 비트 OR |
| `.xor()` | 비트 XOR |

---

## (C) 절차 구문

### always_comb / always_ff / always_latch

**always_comb** (IEEE 1800-2017 §9.2.2.2):
- 묵시적 sensitivity list: 블록 내에서 읽히는 모든 신호 자동 포함. LHS 신호는 제외.
- 시간 0에 자동 1회 실행 (`always @*`는 변화 대기 후 시작).
- LHS 신호에 단일 드라이버 강제 — 다른 always/assign이 동일 변수에 쓰면 컴파일 에러.
- 시뮬레이션에서 무한 루프 감지: 블록이 자기 자신을 재트리거할 때 경고.

**always_ff** (§9.2.2.4):
- 클록 엣지 기반 레지스터 모델링용.
- 정확히 하나의 이벤트 제어 `@(...)` 만 허용.
- 블로킹 타이밍 제어(`#delay`, `@event` 추가) 금지.
- 합성 툴이 플립플롭으로 추론.

**always_latch** (§9.2.2.3):
- 레벨 감지 래치 모델링용.
- always_comb와 동일한 묵시적 sensitivity 규칙.
- 불완전한 조건 분기(래치 추론)를 의도적으로 허용 — 합성 툴 경고 억제.

### unique / priority case·if

**unique case/if**: 런타임에 정확히 하나의 분기만 참이어야 함. 위반 시 런타임 경고(툴 의존).
합성 힌트: 툴이 입력이 상호 배타적임을 알고 인코더 로직을 단순화할 수 있음.

**priority case/if**: 위에서 아래로 순차 평가. 첫 번째 일치 분기 실행.
런타임에 어떤 분기도 실행되지 않으면 경고(완전성 기대).
합성 힌트: 우선순위 인코더 추론.

### foreach

다차원 배열 전체 순회. 복수 인덱스를 쉼표로 나열.

```systemverilog
int arr [3][4];
foreach (arr[i, j])
    arr[i][j] = i * 4 + j;
```

### do-while

최소 한 번 실행 보장.

```systemverilog
int i = 0;
do begin
    i++;
end while (i < 10);
```

### void function + return

반환값이 없는 함수. 중간 탈출에 `return` 사용 가능 (값 없이).

```systemverilog
function void check_range(int val);
    if (val < 0) return;
    $display("in range: %0d", val);
endfunction
```

### ref / const ref 인자

**ref**: 참조 전달. 원본 직접 수정. 대용량 배열 전달 시 복사 비용 0.
**const ref**: 참조 전달 + 쓰기 금지. 수정 시도 시 컴파일 에러. 읽기 전용 대용량 인자에 최적.

```systemverilog
function automatic void add_all(ref int arr[], output int total);
    foreach (arr[i]) total += arr[i];
endfunction

function automatic void print_all(const ref int arr[]);
    foreach (arr[i]) $display(arr[i]);
    // arr[0] = 0;  // 컴파일 에러
endfunction
```

`automatic` 키워드와 함께 써야 재진입(re-entrant) 보장.

---

## 검증 결과 요약

| 출처 | 검증 방식 | 상태 |
|------|----------|------|
| chipverify.com — enum, 2-state vs 4-state | WebFetch 직접 확인 | 일치 |
| verilogpro.com — always_comb/ff, struct/union | WebFetch 직접 확인 | 일치 (single-driver 설명 확인) |
| vlsitrainers.com — tagged union 런타임 에러 | WebFetch 직접 확인 | 일치 |
| vlsiworlds.com — ref/const ref | WebFetch 직접 확인 | 일치 |
| sagar5258.blogspot.com — array 메소드 | WebFetch 직접 확인 | 일치 |

---

## Sources

- IEEE 1800-2017 §6 (Data types), §7 (Aggregate types), §9 (Processes), §12/§13 (Functions/tasks)
- https://chipverify.com/systemverilog/systemverilog-enumeration
- https://chipverify.com/systemverilog/systemverilog-quick-refresher
- https://www.verilogpro.com/systemverilog-always_comb-always_ff/
- https://vlsitrainers.com/system-verilog-union-packed-unpacked-and-tagged/
- https://vlsiworlds.com/system-verilog/argument-passing-and-the-const-keyword-in-systemverilog/
- https://sagar5258.blogspot.com/2017/09/array-manipulation-methods-in.html
- https://verificationguide.com/systemverilog/systemverilog-associative-array/
- https://www.verilogpro.com/systemverilog-unique-priority/
