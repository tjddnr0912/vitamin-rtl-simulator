# 02 · SystemVerilog 배열

IEEE 1800-2017 §7 기준. Verilog는 1차원 정적 배열(메모리)만 지원했으나 SV는 다차원 packed,
dynamic, associative, queue 4가지를 추가했다.

---

## 5가지 배열 종류

| 종류 | 선언 형태 | 크기 결정 | 키 타입 | 합성 |
|------|----------|----------|---------|------|
| Packed | `logic [3:0][7:0] m` | 컴파일타임 | 정수 인덱스 | 가능 |
| Unpacked | `int arr [0:7]` | 컴파일타임 | 정수 인덱스 | 가능(단순) |
| Dynamic | `int d[]` | 런타임 `new[N]` | 정수 인덱스 | 불가 |
| Associative | `int aa[string]` | 자동 해시 | 임의 타입 | 불가 |
| Queue | `int q[$]` | 런타임 push/pop | 정수 인덱스 | 불가 |

---

## Packed 다차원 배열

선언 범위가 타입 뒤(식별자 앞)에 위치 — 연속 비트 벡터로 매핑된다.

```systemverilog
logic [3:0][7:0] matrix;   // 4 × 8비트 = 32비트 연속 벡터
matrix[2][3]               // 3번째 바이트의 4번째 비트
logic [31:0] raw = matrix; // 전체 벡터로 접근 가능
```

비트 폭 계산: 모든 packed 차원의 곱. 합성 시 단일 와이어 집합으로 처리된다.

---

## Unpacked 배열

선언 범위가 식별자 뒤에 위치 — 요소 사이 연속성 보장 없음.

```systemverilog
int arr [0:7];     // 정수 8개 (Verilog 스타일)
int arr2 [8];      // 동일 — SV 단축 표기
int mat  [4][8];   // 4 × 8 다차원 (SV 추가)
```

Verilog의 1차원 메모리 선언과 동일하나 SV에서 다차원 확장.

---

## Dynamic Array

런타임에 크기를 결정하는 unpacked 배열. `new[N]`으로 할당.

```systemverilog
int dyn[];
dyn = new[8];            // 8개 할당, 기본값 0
dyn = new[16](dyn);      // 16개로 재할당, 기존 내용 복사
dyn.delete();            // 전체 해제 (size → 0)
int s = dyn.size();      // 현재 크기
```

- `new[N]` 없이 접근하면 런타임 null 접근 에러
- 재할당 시 `(dyn)` 복사 인자를 생략하면 기존 내용은 버려진다

---

## Associative Array

키-값 해시 맵. 선언 시 메모리 비할당. 드문 주소 공간 또는 유연한 키 타입에 적합.

```systemverilog
int aa[string];    // 문자열 키
int aa2[int];      // 정수 키
int aa3[*];        // wildcard — 임의 정수 표현식 키
```

### 쓰기 / 읽기

```systemverilog
aa["alpha"] = 1;
int v = aa["alpha"];
```

### 메소드

| 메소드 | 동작 |
|--------|------|
| `.exists(key)` | 키 존재 여부 → 1/0 반환 |
| `.delete(key)` | 특정 키 삭제 |
| `.delete()` | 전체 삭제 |
| `.num()` | 엔트리 수 반환 |
| `.first(ref key)` | 첫 번째 키를 key에 저장. 비어있으면 0 반환 |
| `.last(ref key)` | 마지막 키를 key에 저장 |
| `.next(ref key)` | key 다음 키를 key에 덮어씀. 마지막이거나 비어있으면 0 반환 |
| `.prev(ref key)` | key 이전 키를 key에 덮어씀 |

```systemverilog
string k;
if (aa.first(k)) begin
    do begin
        $display("%s = %0d", k, aa[k]);
    end while (aa.next(k));
end
```

---

## Queue

양방향 동적 FIFO. 선언 시 크기 제한을 `[$:N]`으로 지정 가능 (생략 시 무제한).

```systemverilog
int q[$];         // 무제한 큐
int q2[$:7];      // 최대 8개 큐
```

### 메소드

| 메소드 | 동작 |
|--------|------|
| `.push_back(item)` | 뒤에 요소 추가 |
| `.push_front(item)` | 앞에 요소 추가 |
| `.pop_back()` | 뒤 요소 반환 + 제거 |
| `.pop_front()` | 앞 요소 반환 + 제거 |
| `.insert(idx, item)` | 인덱스 idx 위치에 삽입 |
| `.delete(idx)` | 인덱스 idx 요소 삭제 |
| `.delete()` | 전체 삭제 |
| `.size()` | 현재 요소 수 반환 |

```systemverilog
int q[$];
q.push_back(10);
q.push_front(5);
q.insert(1, 7);          // [5, 7, 10]
int v = q.pop_front();   // v=5, q=[7, 10]
```

큐는 배열 슬라이스 문법으로도 접근 가능: `q[0:2]` → 인덱스 0~2 요소.

---

## 배열 메소드 라이브러리

dynamic 배열, unpacked 배열, 큐에 공통 적용된다.

### 정렬·재배열

배열 원소를 직접 수정한다.

| 메소드 | 동작 | `with` 절 |
|--------|------|----------|
| `.sort()` | 오름차순 정렬 | 선택 (키 표현식) |
| `.rsort()` | 내림차순 정렬 | 선택 |
| `.reverse()` | 순서 역전 (값 변경 없음) | 불가 |
| `.shuffle()` | 무작위 섞기 | 불가 |

```systemverilog
int a[] = '{5, 3, 8, 1};
a.sort();                         // {1, 3, 5, 8}
a.rsort();                        // {8, 5, 3, 1}
a.sort with (item % 3);           // 나머지 기준 오름차순
```

### 위치 탐색

`with` 절이 필수이며 반환값은 항상 큐다. 조건 불일치 시 빈 큐 반환.

| 메소드 | 반환 |
|--------|------|
| `.find with (cond)` | 조건 만족 요소 큐 |
| `.find_first with (cond)` | 첫 번째 요소 큐 |
| `.find_last with (cond)` | 마지막 요소 큐 |
| `.find_index with (cond)` | 조건 만족 인덱스 큐 |
| `.find_first_index with (cond)` | 첫 번째 인덱스 큐 |
| `.find_last_index with (cond)` | 마지막 인덱스 큐 |

```systemverilog
int a[] = '{1, 5, 3, 8, 2};
int found[$] = a.find with (item > 4);   // {5, 8}
int idx[$]   = a.find_index with (item > 4); // {1, 3}
```

### 축소

배열 전체를 스칼라 하나로 축소한다. `with` 절로 요소를 변환한 후 축소 가능.

| 메소드 | 동작 |
|--------|------|
| `.sum()` | 전체 합 |
| `.product()` | 전체 곱 |
| `.and()` | 비트 AND |
| `.or()` | 비트 OR |
| `.xor()` | 비트 XOR |

```systemverilog
int a[] = '{1, 2, 3, 4};
int s = a.sum();                    // 10
int s2 = a.sum with (item * 2);     // 20 (각 요소 2배 후 합산)
```

---

## Sources

- IEEE 1800-2017 §7 (Aggregate data types)
- chipverify.com/systemverilog/systemverilog-associative-array
- vlsiverify.com/system-verilog/associative-array-in-systemverilog/
- verificationguide.com/systemverilog/systemverilog-queue/
- sagar5258.blogspot.com/2017/09/array-manipulation-methods-in.html
- verificationguide.com/systemverilog/systemverilog-array-ordering-methods/
