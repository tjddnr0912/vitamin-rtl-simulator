# Design — dynamic array / queue / assoc array 스토리지 (Phase-2 관문 (C), v5 bump 선행 문서)

> **2026-06-10 · §F 진입 시퀀스 ②.** v5 format bump를 한 번으로 끝내기 위한 사전 설계.
> 이 문서가 확정한 IR 형상 목록이 곧 v5 diff의 전부다 — 여기 없는 형상 변경이
> bump 후에 또 필요해지면 설계 실패로 간주하고 문서부터 갱신할 것.

## 0. v5 일괄 범위의 재분류 (이 문서의 첫 결정)

| §F 항목 | 당초 분류 | 확정 분류 | 근거 |
|---|---|---|---|
| (A) NBA delayed write `q <= #d rhs` | bump 필수 | **bump 필수 — v5 포함** | `NonblockingAssign`에 delay 필드(아래 §5) |
| (B) named event `->`/`@(ev)` | bump 필수 | **sim-ir 무변경으로 강등 — v5 제외** | **카운터 desugar**: `event e` → 64-bit Reg(init 0), `->e` → `e = e + 1`(BlockingAssign), `@(e)` → 기존 net AnyEdge 센스티비티. 같은-슬롯 이중 trigger도 카운터 증가라 prev/cur 비교에서 변화 보장(1-bit 토글의 0→1→0 누락 함정 회피). 표현식에서의 event 읽기/쓰기는 elaborate `event_nets` 집합으로 loud 거부. 잔여 비용 = AST decl kind 1개(.vu flip — (D)와 일괄). 동결 `WaitCause::Named`/`WakeCond::NamedEvent`는 예약-미사용으로 유지(제거 불가·무해) |
| (C) dynamic storage | bump 필수 | **bump 필수 — v5 코어** | 아래 전체 |
| (D) interface | 무변경(스파이크 GO) | 무변경 — `.vu`만 | [interface 스파이크](2026-06-10-interface-flattening-spike.md) |

**⇒ v5 (SimIr) bump = (A) + (C) 형상만. `.vu`(AST) flip = (B)+(D)+(C 문법) 일괄 1회.**

## 1. (C) 스코프 — MVP 컷

| 지원 (v5 구현 대상) | 명시 제외 (loud, 후속) |
|---|---|
| dynamic array: `int d[];` `d = new[n];` `new[n](d)`(복사) `d[i]` r/w `d.size()` `d.delete()` | `d[i:j]` 슬라이스, 다차원 dynamic |
| queue: `int q[$];` `q[i]` r/w, `q[$]`, `push_back/push_front/pop_back/pop_front/size/delete()` | `insert/delete(i)`(후속 가능), bounded queue `[$:N]`, 슬라이스 |
| assoc: `int a[int];`(정수 키 ≤64bit) `a[k]` r/w `a.exists(k)` `a.delete(k)`/`a.delete()` `a.num()` | **string 키**, `first/next/last/prev`(SysFunc 추가=차기 bump 묶음), assoc-`foreach`(first/next 의존) |
| **`foreach (arr[i])` — dyn array/queue ✅ 2026-06-11**: 파서가 `size()` 카운팅 루프로 desugar(신규 AST/IR 0 — 합성 인덱스명+body rename으로 IEEE 인덱스 지역성 보존, iverilog 차분) | queue `insert(i,v)`/`delete(i)`·bounded `[$:N]` 실동작(SysTask 추가 또는 사이드카 — 차기 bump/사이드카 묶음) |
| 원소 타입: 임의 폭 packed(4-state `Value` 보존), real | 원소가 struct/another-dyn인 중첩 |

## 2. 스토리지 모델 — handle-net + 엔진 힙

**원칙: 동적 객체는 BitPacked 평탄 스토어에 들어가지 않는다.** NetVar는 "핸들 선언"만:

- IR: `NetKind` += `DynArray | Queue | Assoc`. `NetVar.width` = **원소 폭**, `array_len = 0`,
  `init` = 빈 BitPacked. msb/lsb는 원소의 것.
- 엔진: `SimState.dyn_heap: BTreeMap<u32 /*NetId*/, DynObj>` (lazy-init; 핸들 net 수만큼).
  ```rust
  enum DynObj {                       // 엔진 내부 — 직렬화 안 됨(런타임 상태)
      DynArray { elems: Vec<Value> },         // new[n]만 크기 변경
      Queue    { elems: VecDeque<Value> },
      Assoc    { map: BTreeMap<i64, Value> }, // 정수 키, BTree = 순회/덤프 결정성
  }
  ```
- **결정성 계약**: 슬롯 키 = NetId(선언 순서), 주소/capacity는 어떤 표면(stdout/VCD/진단)에도
  비노출, assoc 순서 = BTree 키 순서. 3-OS byte-identity 불변식 유지.

## 3. v5 IR 형상 diff (전체 목록 — 이것이 bump의 전부)

| 타입 | 변경 | 용도 |
|---|---|---|
| `NetKind` | += `DynArray, Queue, Assoc` | 핸들 선언 |
| `NonblockingAssign` | += `delay: Option<u32 /*ExprId*/>` | (A) NBA transport delay |
| `SysFuncId` | += `DynSize, QPopBack, QPopFront, AssocExists, AssocNum` | 값-반환 메서드. `q[$]` = elaborate가 `DynSize-1`로 desugar. pop류는 side-effecting → P9 allow-list 제외(VM은 해당 바디 interp fallback) |
| `SysTaskId` | += `DynNew, DynDelete, QPushBack, QPushFront, AssocDeleteKey` | stmt 메서드. `DynNew(handle, n [, src])`, `delete()` 공용(`DynDelete`), assoc `delete(k)`는 키 인자 |
| `Expr::Signal`/`LvalChunk` | **무변경** — `word: Some(idx)`를 dyn 핸들에 재사용 | `d[i]`/`q[i]`/`a[k]` r/w. 엔진이 NetKind로 평탄배열 vs 힙 라우팅 |

> 검토 각주: SysTask/SysFunc에 "DynMethod 1개 + method-id 인자" 안은 기각 — bump 비용이
> 동일한 이상 명시 variant가 doc-15 진단·P9 분류·코드 리뷰 전부에서 우월.

## 4. 의미론 (IEEE 1800 §7, 구현 시 오라클 라이브 필수)

- **OOB/미존재 read**: dyn/queue 범위 밖, assoc 미존재 키 → **원소-폭 X**(4-state) + 런타임
  warn(net당 1회 — 진단 폭주 방지). X/Z 인덱스 → 동일 (정적 배열 sentinel 모델 재사용).
- **OOB write**: dyn array 범위 밖 → 무시+warn. queue는 **`q[size()] = v`가 push_back 동등
  (IEEE §7.10.1, 합법-무음·1개 성장)**, 그 너머만 무시+warn — 당초 "q[size] write 무시"는
  iverilog 라이브 오라클(size 1→2 성장 확인)로 **구현 시 정정(2026-06-11)**. assoc write
  미존재 키 → 원소 생성.
- **`new[n]`**: 기존 원소 보존 없는 형(`d = new[n]`)은 전부 X로, 복사형 `new[n](d)`는 prefix 복사.
  **n이 X/Z → 빈 배열 + warn-once; n==0은 합법-무음**(IEEE §7.5.1 — 구현 시 정정, 2026-06-11).
  n > 1<<24(=elaborate MAX_ARRAY_LEN과 동일 캡 클래스) → **경고 후 클램프**(no silent caps).
- **pop on empty**: 원소-폭 X + warn (iverilog/상용 모두 warn류 — 라이브 확인 후 핀).
- **assoc 오라클 주의**: iverilog 13.0은 assoc array 선언 자체를 거부(파스 에러) —
  ⑤의 의미론은 **hand-IEEE 핀**(§7.8.6 invalid-index, §7.9 메서드; 2026-06-11 라이브
  확인). `exists(X키)`=0+warn-once, `delete(미존재 키)`=무음 no-op은 이 핀의 일부.
- **이벤트/센스티비티**: dyn 핸들은 `@()`/wait/VCD 대상 불가 — elaborate loud 거부.
  내용 변경은 net dirty 채널에 **참여하지 않음**(조합 re-eval 트리거 없음) — 절차 코드 전용.
- **VCD**: 미덤프 — **구현됨(③)**: 두 declare 경로 모두 dyn 핸들 skip(vcd_id=None →
  initial dump·change record가 구조적으로 0). 1회 정보 진단은 보류(skip이 무해해 노이즈 판단;
  필요시 doc-07에 추가). 진단 코드 = **`VITA-W4020 W-RUN-DYN-DEGRADE`**(warn-once per net,
  doc-15 등재 — X-size new/캡 클램프/향후 OOB·empty-pop 공용).

## 5. (A) NBA delayed write — 같은 bump의 동승자

- 형상: `NonblockingAssign { lhs, rhs, delay: Option<u32> }` (ExprId — v4 런타임 delay 모델 재사용).
- 엔진: 실행 시 `d` 평가(suspension-time 규약과 동일) + RHS/인덱스는 **지금** 샘플 →
  wheel의 `t+d` NBA-region에 **값-운반 이벤트** `schedule_nba_at(t+d, lhs, value)`.
  겹침 활성화는 각자 자기 캡처 값을 운반(transport delay) — ⑤에서 정적 capture를 기각했던
  바로 그 사유의 해소. `delay: None` = 현행과 byte-동일.
- 차분: iverilog `q <= #d v` 겹침 케이스 오라클 라이브 → diff 핀. E3009 loud 제거.

## 6. 절차 — v5 bump 체크리스트 (v4 절차 재사용)

1. 형상 diff(§3) 일괄 적용 + `CURRENT_FORMAT_VERSION = 5`.
2. `REGEN_GOLDEN=1`로 canonical txt·RON registry·`.velab` corpus 재생성(스위치 기존).
3. doc-17 lowering 표 + doc-15 코드(신규 warn/error) + doc-14 trailer 문서 갱신.
4. 구현 증분 순서: ~~①bump PR~~✅(`e7f08e8`) ~~②(A) NBA-delay~~✅(`1617980`) —
   ③dyn array(**3a: heap/new/size/delete ✅ 2026-06-11** — `dyn_heap`+`DynObj`, hand-built
   SimIr 시임으로 테스트(문법은 ⑥); **3b: 인덱스 r/w 라우팅 ✅ 2026-06-11** — `read_net`/
   `write_chunk` 깔때기에 `dyn_is_handle` 비트맵 1로드 라우팅(정적 경로 무세금 확인), OOB/X-idx
   read=원소폭 X·write=무시(+W4020 once), **NBA-to-element는 같은 write_lvalue 깔때기라
   by-construction 라우팅 — 바운드는 APPLY 시점 크기 기준**(스케줄-적용 사이 resize는 IEEE
   미규정 — 결정적 규칙으로 핀), dirty 채널 비참여=설계 §4 그대로)
   **④queue ✅ 2026-06-11** — `DynObj::Queue{VecDeque}`; push=SysTask 디스패치(원소형 cast
   §5.5, cap 초과=warn+drop), 인덱스 r/w는 3b 깔때기 공유(+`q[size()]`=append 합법-무음 —
   §4 정정 참조), **pop=`StmtEffect::QPop` 문장-레벨 인터셉트**(side-effect는 WRITE phase —
   P7a read-phase 순수성 유지; lvalue offsets는 pop 전 resolve로 핀): Kernel에
   `k_queue_pop_rhs`/`k_queue_pop` 2메서드, **pop-rhs 바디는 `is_codegen_able` 제외**(VM은
   interp fallback — 설계 §3 각주 그대로; teeth는 VM byte-parity 테스트로 검증), 그 외
   배치(NBA rhs·중첩 식)의 pop은 eval 순수-arm에서 X+W4020(미-pop, ⑥이 loud-reject 예정),
   pop SelfWidth=원소형(signed byte −1→int −1 / unsigned 255→255, iverilog 라이브),
   `q[$]` desugar(`DynSize-1`) 시임 테스트 포함(빈 큐 → −1 → OOR 센티넬 → X+warn).
   **⑤assoc ✅ 2026-06-11** — `DynObj::Assoc{BTreeMap<i64,Value>}`. **키 도메인 =
   signed i64 전역**(음수·>u32 키 합법)이라 기존 `(u32,u32)` offsets 쌍에 실을 수 없음
   (모든 u64 비트패턴이 합법 키 → in-band 센티넬 불가) → **`Offsets::AssocKey(Option<i64>)`
   variant 신설** + `k_write_lvalue`/`write_lvalue` ABI를 pair-slice→`&Offsets`로 변경
   (blocking·NBA·delayed-CA·QPop·VM 전부 같은 깔때기 = by-construction 일관). READ는
   eval Signal arm이 u32 word coercion **전에** `is_assoc` 분기(핸들 비트맵 short-circuit,
   정적 경로 무세금) → `assoc_key`(키 expr을 자기 signedness로 64-bit 확장, >64 절단=§5.5,
   X/Z·real=invalid `None`) → `assoc_read`. exists/num=**순수** eval arm(힙 불변 — pop과
   달리 인터셉트 불요, VM parity by-construction), `delete(k)`=SysTask 디스패치(미존재
   키=무음 no-op §7.9, X/Z 키=W4020), `delete()`=DynDelete 공용. native-eval
   `LoadIndexed`는 **Assoc bail**(u32 도메인이라 음수 키가 인터프리터와 발산할 것 —
   컴파일 거부로 oracle-bound 유지). concat-lvalue 안의 assoc chunk = pair 경로로
   폴스루 → dyn_write가 loud 무시(⑥이 reject). ⚠️ **iverilog 13.0은 assoc 선언
   자체를 거부**(`[int]`/`[longint]`/`[*]` 전부, 라이브 확인 2026-06-11) — dyn 3종 중
   유일하게 **hand-IEEE 핀**(§7.8/§7.9, expression-force 선례)으로 의미론 고정.
   **⑥front-end 일괄 ✅ 2026-06-11(722 green, .vu flip 1회)** — (C) 문법: lexer `$`
   토큰(LoneSigil에서 분리)·`Dim::{Dyn,Queue(Option bound),Assoc(AssocKey)}`·
   `ExprKind::{New,Dollar}`(new는 컨텍스추얼 파스 — V2005의 `new` 식별자는 elaborate
   net-named-new 폴백으로 보존)·parse_dim 4분기(`[*]`=parse-loud)·메서드는 기존
   `Call{HierPath 2seg}` AST 재사용(파서 무변경). elaborate: 핸들 NetVar(array_len 0)
   decl 인터셉트(포트/init/real·event 원소/혼합 dim/bounded=loud), BitSelect r/w를
   정적 체인보다 먼저 dyn 라우팅, `q[$]`=`dollar_subst` save/restore로 `DynSize-1`
   치환(`q[$-1]` 산술 지원 — iverilog는 이 문법 자체를 거부, hand-IEEE), `d=new[n]`/
   `x=q.pop_*()`만 BlockingAssign 특수형(그 외 배치 전부 E3009 loud), 메서드 표
   (size/num/exists/push/pop/delete(k)) kind-체크 디스패치, 전 핸들 오용(whole r/w·
   센스티비티·이중 dim) loud. iverilog 차분: dyn/queue 레인 라이브 핀(`q[$-1]`·assoc
   제외 — 후자는 hand-IEEE). e2e 13 테스트. (D) interface: 스파이크 설계 그대로 —
   `TopItem::Interface(ModuleDecl 재사용)`+`ModuleItem::Modport`+`AnsiPort.iface`,
   parse_module_like 공용 파서, elaborate `ifaces`(owned clone)/`iface_insts` 레지스트리,
   **인스턴스는 nets 단계(4c) 조기 평탄화**(부모 body의 `i.sig`가 pass 7에서 해소돼야
   — 멱등 가드로 generate-경유 늦은 경로와 공존), 포트 바인딩=심볼 aliasing(BTreeMap
   prefix-range 복사, cont-assign 0개·net 0개), resolve_net 다중-세그=dot-join 스코프
   워크(실패시 기존 deferred loud 유지), modport=존재 검증+수용(방향 강제는 후속).
   **iverilog 13.0은 interface-typed port도 거부 — hand-IEEE 핀**(§25). e2e 8 테스트
   (alias 동일-net 증명=엣지 센스티비티 관통, MVP cut loud 레인 포함). SimIr 무변경
   (frozen 0줄, format_version 5 유지), `.vu` 해시 재핀 1회(dyn+iface 일괄).
   각 증분은 TDD + iverilog 라이브 오라클(§4의 warn 문구류·assoc·interface 전체는 hand-IEEE 핀).
5. P9/native-eval: dyn 관련 Expr/Stmt·NBA-delay는 allow-list 제외로 시작(전부 interp) —
   P5 차분 게이트가 자동으로 안전망.
