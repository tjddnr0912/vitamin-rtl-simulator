# ROADMAP — 잔여 과제 (vitamin)

> **이 문서 = 전방(남은 것)-전용.** 완료 항목의 상세 로그(§4.5.x)는 [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)에, 옛 §번호(구 §0~§7) 원문은 [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md)에 있다(둘 다 §번호 보존). 이력 내러티브 = [DEVLOG.md](DEVLOG.md), 상위 스냅샷 = [REMAINING_WORK.md](REMAINING_WORK.md), 실행 큐 = `LOOPROMPT.md` NEXT(로컬 dev-meta), SPEC 정본 = `docs/preview/`.
>
> **기준선(2026-07-28)**: format_version **25** · **4717 tests green** · 3-OS CI green · MsgCode **59** · **MSRV 1.85**. 최신 = **§4.5.249**(외부 round-20 §6 진단 위치 + §4.11 같은 이름 동적 로컬). 직전 = **§4.5.248**(외부 round-20 8 가족 — fork-arm 블록 로컬·queue 관용구·named arg·`$sformatf`). 직전 = **§4.5.247**(§4.5.246 회귀 수정). 직전 = **§4.5.246**(inner NET shadow — 마지막 ①-급 해소). 그 이전 슬라이스(§4.5.222~245)의 한 줄 요약과 상세는 전부 **[ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)**(인덱스 = 파일 상단, `#### 4.5.<N>` 검색) — 이 문서는 **전방 전용**이므로 완료 서사를 두지 않는다.
>
> **운용 규칙**: 완료 항목은 **즉시 이 문서에서 제거**하고 ARCHIVE로 옮긴다 — 취소선 잔류가 이 파일을 106KB까지 불린 원인이다(잔여가 남은 항목만 "RESOLVED(§x·상세=ARCHIVE) — 잔여 …" 한 줄로 유지).  슬라이스 완료 시 → 상세 로그를 ARCHIVE "완료 슬라이스 로그"에 append(§4.5.x 양식·최신이 위), 이 문서의 해당 잔여 항목 삭제. 신규 발굴은 아래 해당 섹션에 1줄로 추가.


## 요약 (스캔용)

| 순 | § | 주제 | 항목 | 오라클 | 키워드 |
|---:|---|---|---:|:--:|---|
| **1** | §0 | correct-support 승격 큐 | 6 | ✓ 4/6 | **T1 잔여까지 전부 완료(§4.5.222~227)** · 남은 것 = T2 real const-fold/generate string/enum label/음수 range 4 + T3 전제조건 2 |
| **2** | §2 | Silent-wrong 잔여 | 42 | ✓ 8 | **폭 인식 상수 접기(3건 동근)** · package real · 구조적 지연 · inner-NET shadow(DEEP) |
| **3** | §6 | G2 OBS 트랙 | 6단계 | 내부 3-way | OBS-2 sva.jsonl → OBS-1 잔여 → R-L4 → OBS-4/5/6 |
| **4** | §3 | Loud→supported 후보 | 35 | ✓ 대부분 | string/heap · 함수/formal · 소형 큐 · VCD fidelity · deep 저우선 |
| **5** | §4 | SVA / 검증 honest-loud | 6 | 일부 無 | empty-match 융합 · N2c · prop-ref skew · N4 clocking · class down-cast |
| — | §5 | perf / 하드닝 | 4 | — | **전부 보류 판정** — 트리거 시만 |
| — | §7 | 조건부 / 장기 | 4 | — | BACKEND · VHDL · VCD-EXT · MVP-CUT (정확성과 직교) |
| — | §8 | 비계획 | 1 | — | 영구 비목표(DEFPARAM·IMPLICIT-NET·OOS) |

> 🔴 = 열린 silent-wrong(정본 최우선). 취소선/RESOLVED 항목은 **잔여가 있을 때만** 한 줄로 남고 상세는 ARCHIVE에만 둔다.
>
> **순서 주의**: 정본 우선순위(§1)는 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported`인데, 위 표의 1·2위는 **오너 지시로 §0(②)이 §2(①) 앞**에 있다. §0를 먼저 해도 §2의 ①-급이 사라진 것은 아니다.

## 0. correct-support 승격 큐 (2026-07-25 전수 재그라운딩 — **오너 지시로 최상위**)

> **목적**: 지금까지 correct-or-loud로 **LOUD 유지**한 항목 중 *실제로 구현 가능한 것*을 골라 correct-support로 올린다. 아래는 §3/§4 전체를 훑고 **12개 후보를 iverilog로 직접 재현**해 (a)아직 loud인지 (b)오라클이 있는지 확인한 결과다. 재확인에서 **4건이 stale**로 드러나 아래 "정정" 항목으로 이동했다.
>
> **순위 근거**: 전부 iverilog 오라클 有 = §1 우선순위 ②(additive·저위험). ~~T1은 한 가족이라 머신러리가 공유된다~~ — **§4.5.222 실측이 이 전제를 기각했다**(아래 T1 머리말). T2는 서로 독립적이라 개별 슬라이스.

**~~T1 — string-array~~ 전부 RESOLVED** (§4.5.219 decl-init · §4.5.220 전제조건 dyn byte-select · §4.5.222 runtime index + `foreach` · §4.5.223 queue-of-string · §4.5.224 multi-dim · §4.5.225 frame-local · §4.5.226 hier read · **§4.5.227 잔여 11항목**). 상세=ARCHIVE.

> **정정(기록 보존)**: 이 큐는 T1을 "한 가족·머신러리 공유"로 묶었으나 **틀린 전제였다** — 근인이 4갈래였고, 6·7은 **DYN 배열에서도 똑같이 loud**라 라우팅과 무관했다. 각각 전용 슬라이스로 갔다. 큐를 묶을 때 근인을 측정하지 않으면 이렇게 된다.

**T1 잔여 = 없음 (§4.5.227·2026-07-27).** 위 표의 11항목 전부 correct-support. 근인은 4갈래였다 —
**geometry**(1·2·3·4·5·7: 라우팅이 zero-based를 *가정*하던 것을 `flatten_word`/`lower_fixed_foreach_step`로
*적용*으로 바꿈) · **bound**(6: 엔진 bound는 원소타입 무관, 선언 패턴이 `Queue(None)`만 받던 것) ·
**hierarchical**(7·8·10: deferred read/write가 주소 규칙 하나를 공유하게) · **per-activation**(9: §4.5.171
frame-local dyn 배열 fatal → 활성화별 stash/restore) · **SoA**(11: whole-element를 멤버별로 fan-out).
상세=ARCHIVE §4.5.227.

**T1에서 발굴한 잔여 4건 = 전부 RESOLVED (§4.5.228·round-20).** generate 스코프 라우팅 ·
fork-arm 재개(🔴) · 동시 활성화 dyn 배열 · 음수 하한 unpacked 배열. 상세=ARCHIVE §4.5.228.

**의도적 loud(갭 아님)**: fixed 배열에 `new[]`(iverilog도 거부) · multi-dim partial 인덱스 `s[0]`(iverilog도 거부·조용한 오원소 방지) · cross-type SoA whole-element 복사(멤버 대응 보장 없음).

**오라클 주의 — iverilog 결함 2건(vita가 IEEE 정답)**: ① string **배열 원소**의 `.len()`이 문자열 길이가 아니라 **배열 크기**를 낸다(`string s[5]; s[0]="abcdefg"` → iverilog 5, vita 7; 같은 텍스트를 스칼라에 넣으면 iverilog도 7). ② 동시 fork 활성화가 automatic string 배열을 공유한다(`A!` 대신 `A!!`). ③ 같은 fd 에 `$fmonitor` 를 두 번 걸면 **누적**해 둘 다 찍는다 — 자기 자신의 싱글턴 `$monitor` 와 모순(vita 는 destination 별 replace). ④ 빈 string **배열 원소**를 `%s` 로 찍으면 공백 1칸(스칼라는 빈 문자열). 전부 회귀 테스트로 핀 고정.

**T2 — 독립 항목 (오라클 ✓·각자 전용 슬라이스)**

8. ~~`real` const-fold 전면 미지원~~ **RESOLVED**(§4.5.232) — 실수 산술·alias·체인·정수 승격 전부 correct-support. **잔여(의도적 loud)**: 실수를 정수 문맥에(폭/`$clog2`/replication/정수 localparam) — **i64 twin 등록은 시도했다가 철회**(twin 이 정수 도메인에 real 식을 열어 generate 분기 오선택 등 5건 silent-wrong; §11.8.1 순서를 아는 site 가 `param_real_value` 하나뿐) · ~~generate 스코프/제어식의 real~~ **전부 RESOLVED**(§4.5.241/242). **잔여 없음** — case scrutinee 의 real 은 **iverilog 도 거부**하므로("Cannot evaluate genvar case expression") 갭이 아니라 **비목표**(§4.5.243 확인·`generate_case_real_nongoal.rs` 가 정수형 동작과 함께 핀) · `1.0/0.0` · 실수 미정의 연산자 · 실수 override.
9. ~~generate/interface 스코프 string decl-init~~ **RESOLVED**(§4.5.228) — 근인은 `allow_string_init` 플래그가 아니라 decl-time 쓰기가 모듈 스코프 pending 리스트로 새던 것. queue/dyn decl-init·generate 내 block-local 도 같이 열림.
10. ~~sized-literal enum label → enum-method~~ **RESOLVED**(§4.5.234) — 신규 `const_lit_based` 를 enum 라벨만 opt-in. **잔여**: 파서 폴드는 **절단이 필요한 리터럴을 거부**한다(unsized `'h1FFFFFFFF`·mis-sized `4'hFF`) — elaborate 의 `parse_int_literal` 과 폭 규칙이 달라 값이 갈릴 수 있는 입력을 아예 안 받는 설계이며, 근본 해소는 §4.5.233 의 안 ①(literal 파싱 공유 크레이트 분리)뿐.
11. ~~음수 range bound~~ **RESOLVED**(§4.5.228) — plain net·multi-packed inner(**후자는 silent 였다**)·배열 원소·VCD `$var` 범위까지. 잔여 = **PART select**(`x[1:-2]`, 정직한 loud·바운드 접기가 unsigned) · **포트/formal**(warn+clamp 유지·의도적 opt-in 비대칭).

**T3 — 전제조건 필요 (즉시 착수 대상 아님)**

12. ~~`$fmonitor`/`$fstrobe`~~ **RESOLVED**(§4.5.228·format 25) — 동결 `Monitor`/`Strobe` id 재사용 + `file_directed_stmts` 사이드카. 모니터는 **destination 별**로 유지된다.
13. `case (x) inside {…}` — **no-oracle**(iverilog 13.0이 `case inside`/`inside` op/array reduction 전부 거부) → hand-IEEE + 내부 차분.

**정정 — 재그라운딩에서 stale로 판명(§3에서 삭제, 비목표)**

- ~~package `function string`~~ · ~~package control-flow 함수~~ — 둘 다 **이미 동작**(`hi` / `9` 확인).
- ~~generate 스코프 queue/dyn decl-init~~ — queue **이미 동작**(`4` 확인). 잔여는 string decl-init뿐(위 9번).
- ~~`always @(*)` string concat 조용히 drop~~ — **오진**. vita는 `[abcd]`로 **정확**하고 iverilog가 빈 문자열을 낸다. 명시 `@(a,b)`는 vita가 정직하게 loud. §4.5.217 리포트의 이 줄은 잘못 기록된 것.

> **주의(정직한 순위 고지)**: 프로젝트 정본 우선순위(§1)는 **① 오라클 있는 CRITICAL silent-wrong > ② loud→supported**다. 아래 §0-B의 **inner NET vs outer PARAM shadow**는 여전히 ①-급(silent-wrong)이므로, 위 승격 큐를 먼저 하더라도 그 항목이 사라진 것은 아니다.

## 0-B. NEXT — 재개할 deep-defer follow-on

> **round-18 리포트 8-가족 RESOLVED(§4.5.213·2026-07-24)** — 외부 리뷰어 round-18 리포트의 잔여 8-가족(A/G queue/array-of-non-packable-record + foreach·D automatic-block-local-init·E1 enum-method-on-formal·E2 struct-member-method + string-dyn-element·F1 output-formal-fn-in-loop-cond·F2 severity-in-frame-body·F3 wrapped-dyn-formal) + C1 const-repeat를 correct-support화(hand-IEEE/iverilog 차분). 상세=ARCHIVE §4.5.213.
>
> **C1 part 2: fork-in-frame RESOLVED(§4.5.214·2026-07-24)** — `fork…join[_any|_none]`을 suspendable(framed) task 내부에서 실행하는 것이 "깊은 스케줄러 rework·blast radius=frame 서브시스템 전체"라 correct-or-loud LOUD 유지 중이었으나, 재조사 결과 **단일-스레드 스케줄러 + 기존 `stash_frame_windows`/`restore_frame_windows`가 이미 concurrent children을 parked parent로부터 격리**하고 있어 arm이 부모 frame-local을 안 건드리는 **Case A**(리포트 repro)는 기존 owned-window 모델로 즉시 동작함을 확인 — 신규 인프라가 필요한 건 arm이 부모 frame-local을 read/write하는 **Case B**뿐(interior-mutable arena `WindowSlot::Shared`, dyn_heap/class_heap과 동형). 3-stage(Case A·Case B `join`-all arena·Case B `join_any`/`join_none` refcount)로 전달 + final-review가 fork arm 내부 `return`의 silent frame-corruption 회귀를 잡아 즉시 loud화. format 23 불변. 상세=ARCHIVE §4.5.214.
>
> **§0 NEXT 최상단(2026-07-25 갱신·§4.5.218서 string-array 절반 RESOLVED)** — **inner NET이 outer PARAM/enum-label을 shadow 못 함**: §4.5.218이 string-array side-map은 opt-in shadow walk로 해소했으나 `params`/`param_meta` consumer는 **순서 의존** 때문에 손대지 못했다(초판이 시도했다가 중첩 generate body를 조용히 삭제 — ARCHIVE §4.5.218 S1). 오라클 有(function-local `int W`가 module `localparam W`를 안 가림=vita 4 vs iverilog 9). **전제 = order-INDEPENDENT AST-gathered per-scope name set**(`gather_local_decl_names`/`compute_scoped_block_locals`의 "pure function of the AST" 패턴)—이것 없이 params consumer를 켜면 S1이 그대로 재발한다. 재현·형제 항목(package 변수 clobber·block-local 잔여 2형) 상세 = §2.
>
> 그 외 재개할 deep-defer 항목 없음(round-18 리포트 8-가족 + C1 part1/2 全 RESOLVED). 남은 것은 소형 follow-on(아래) 또는 §1 우선순위 큐(loud→supported/OBS) 재개.
>
> 소형 follow-on(correct-or-loud loud 유지): void-cast of output-formal fn(`void'(getnext())`)·frame-formal array를 nested hier로 forward(OUTPUT/INOUT)·param/call leaf size-cast(`8'(P*a)`, §4.5.212 잔여)·**fork-in-frame 잔여(§4.5.214, 전부 Minor/safe)**: `fork_arms_self_contained`의 resolve-time 재-walk 중복 제거·공유 `enter_task_frame` arm에 load-bearing comment 보강·forking task를 호출하는 fork arm의 elaborate-time reject(현재는 F4004 tie-cap runtime guard로 안전하나 clean E3009가 더 명확)·same-instant zero-delay sibling visibility가 differential-미검증(iverilog 자체가 이 케이스서 스케줄링 특이).

## 0-C. 남은 대형 항목 3건 — 착수 판단표 (§4.5.244 실측)

> 소형·중형 잔여가 §4.5.229~243 으로 소진돼, 남은 것은 **전부 선행조건이 있는 대형 항목**이다. 각각의 **비용·payoff·선행조건**을 실측해 두었으니 다음 착수는 이 표에서 고르면 된다(크기 재추정 금지 — §4.5.233/240 의 교훈).

| 항목 | 비용 | payoff | 선행조건 / 함정 |
|---|---|---|---|
| **A. 파일위치 함수군**(`$ftell`/`$fseek`/`$rewind`/`$ferror`) | **format_version bump 확정** | 中(테스트벤치 파일 I/O) | 신규 `SysFuncId` 변종 = **frozen-root 변경** → SimIr 스키마해시·canonical·RON 골든 **전부 재핀** + 전 `.velab` 무효화. **§4.5.228 의 사이드카 우회는 불가** — `$fmonitor` 는 `$monitor` 의 destination 변종이라 기존 id 재사용이 됐지만, `$ftell`/`$rewind` 는 **의미가 겹치는 기존 id 가 없다**(실측). `$feof`/`$fgetc`/`$ungetc` 는 이미 id 가 있으니 **그것들만 쓰는 범위**라면 bump 없이 가능. |
| **B. literal 파싱 공유 크레이트**(§4.5.234 안 ①) | 中~大(559줄 이동 + 어댑터 + 전 리터럴 재검증) | **小** — 현재 파서가 거부하는 형태는 절단 리터럴(`4'hFF` in `[3:0]`)과 unsized+`s` 인데 **iverilog 도 절단 형태를 거부**한다. 즉 **능력 이득이 거의 없고** 얻는 것은 *두-술어 위험의 구조적 제거*. | `literal.rs` 가 `sim_ir::{BitPacked,ConstRepr,ConstVal}` 에 의존 → 그대로 옮기면 **hdl-parser 가 sim-ir 을 보게 된다**(레이어링 역전). 올바른 분해 = **digit→bits(중립) / ConstVal 패킹(IR)** 2단 분리. |
| ~~C. inner NET shadow~~ **RESOLVED §4.5.246** | — | — | 남은 대형 = A(파일위치군·format bump) · B(literal 크레이트·payoff 작음) |

> **권장 순서 = C > A > B.** C 만이 ①-급이고(§1 우선순위 룰), A 는 format bump 를 감수할 가치가 있을 때, B 는 능력 이득이 거의 없으므로 **다른 이유(두-술어 제거)가 우선순위를 얻을 때**만.

## 1. 착수 우선순위

1. **오라클 있는 CRITICAL silent-wrong** (§2에서 선정) — 항상 최우선.
2. **오라클 있는 loud→supported** (§3 · additive=저위험).
3. **전제조건 충족된 honest-loud 승격** (§4~§5).
4. **G2 OBS 슬라이스** (§6).

현재 NEXT 큐(상세=LOOPROMPT · 스캔용 표 = 문서 상단):

1. **§0 T2 잔여 2건** — `real` const-fold · sized-literal enum label(각자 독립 슬라이스). generate/iface string decl-init·음수 range bound·`$fmonitor`/`$fstrobe`·T1 전부 완료(§4.5.222~228). `real` const-fold 는 §4.5.229 가 남긴 `int'(<real param>)` 바운드의 선행이기도 하다.
2. **§2 오라클-有 silent-wrong** — ~~part-select 바운드 + replication count~~ **RESOLVED**(§4.5.229). 남은 것 = **폭 인식 상수 접기**(위 "상수 폭 잔차" ①②③ 3건이 전부 동근 — 인터프리터 coerce 가 가장 도달성 높음) · package-scope real · **구조적 지연**(§4.5.221이 도달성을 넓혀 우선순위 상향 후보) · real→`input int` formal.
3. **§2 DEEP** — inner NET vs outer PARAM shadow(**선행 = order-INDEPENDENT AST-gathered per-scope name set**; 없이 켜면 §4.5.218 S1 재발) + 형제 항목(package 변수 clobber·block-local 잔여 2형).
4. **OBS-2 sva.jsonl**(R-L6).

## 2. Silent-wrong 잔여 (1건 제외 전부 pre-existing·baseline 동일 — deep defer 또는 기록됨)

> **오라클 있는 것부터 위로.** 아래 🔴 중 A1~A7(오라클 ✓)이 §1 우선순위 ①에 해당하고, 무오라클/soundness 발굴분은 그 아래.

- **🔴 §4.5.221이 도입한 좁은 하강(pre-existing 아님 — 내 책임)**: 계층 real param 이 상수 범위 바운드에 오면 조용히 1-bit. `logic [$clog2(u.R)-1:0]`(u 는 real param R 을 가진 인스턴스) → PRE 는 loud(`E3009`) 였으나 POST 는 **진단 없이 width 1**. 원인 = `count_reads_real_param` 의 `Ident` arm 이 `segments.len()==1` 을 요구해 `u.R` 을 못 봄. **iverilog 도 거부**하므로 무오라클이고 범위는 좁으나 loud→silent 는 하강. **fix 전제** = 바운드 문맥에서의 계층 이름 해석(현재 `nonconst_bound_reason` 은 false-loud 회피 때문에 call/hier 로 안 내려감) — 이름 매칭 근사는 동명 정수 hier param 을 false-reject 할 수 있어 실측 hazard set 없이는 금지(§4.5.218 선례).
- **replication 비대칭(기록)**: 같은 `parameter real R = 3.0` 에서 `{R{'1}}`·`{(R:R:R){1'b1}}` 는 supported(`111`)인데 `{R{1'b1}}`·`{R{2'b10}}` 는 loud. 일관성은 없으나 loud 쪽이 정직함.
- **concat 원소가 정수 산술식이면 바이트가 아니라 워드 폭으로 렌더**(pre-existing·§4.5.222 3-way 실측서 PRE==POST 확인): `{"e", "0"+k}`(k=1) = iverilog `e1` / vita `e\0\0\01`. 스칼라 string·fixed 배열·non-zero-base 전부 동일 발화 = §4.5.134/217의 "string concat 폭" 가족과 동근. §4.5.222가 런타임 인덱스 write라는 **철자 하나를 더 도달 가능하게** 했을 뿐(신규 결함 아님). 올바른 concat(`{s[k],"!"}`)은 iverilog 일치라 blanket 가드는 false-loud → 술어는 **산술 피연산자의 self-width**여야 함.
- ~~🔴 상수-foldable 식을 part-select 바운드로~~ · ~~🔴 replication count 가 안 접히면 0~~ **둘 다 RESOLVED**(§4.5.229·동근이었고 실측은 기록의 2가족이 아니라 **8가족**). 잔여 = 아래 "상수 폭 잔차" 3줄.
- **모듈 스코프 상수식 = 비목표(§4.5.231 그라운딩)**: 예전에 "잔차 ①/③"으로 적어둔 `localparam E=(8'd200+8'd100)>>2`(iverilog 11 / vita 75) 등은 **vita 결함이 아니다**. iverilog 13.0 의 untyped-param 접기가 **세 갈래로 자기모순**임을 실측했다 — ① 같은 식을 `+0` 으로 감싸면 값이 바뀐다(11→75) ② `+`/`*` 는 무제한인데 `<<` 만 32비트 wrap(`32'd1<<32'd33`=0 인데 `32'd100000*32'd100000`=1e10) ③ 그래서 같은 자리에서 문맥 폭 질문의 답이 연산자마다 다르다. **어떤 단일 폭 모델도 iverilog 답을 재현 못 한다 = 오라클 없음** → §4 룰대로 vita 는 **자기일관성**을 지킨다(한 도메인, 균일 적용). teeth = `const_expr_self_consistency.rs`(vita 대 vita: 값 보존 래퍼가 값을 바꾸면 안 되고, 연산자끼리 문맥 폭을 두고 갈리면 안 된다). **잔여 실질 갭은 select 바운드 한 곳뿐** — `v[7:((32'd1<<32'd33)>>32'd30)]` 에서 §4.5.229 가드가 접기를 거부해 조용한 1비트를 남긴다(loud 가 더 정직하나 §4.5.229 의 의도적 decline 동작).
- ~~상수 폭 잔차 ②(상수함수 인터프리터)~~ **RESOLVED**(§4.5.230) — 폭 인식 평가 도입. **잔여**: `$signed`/`$unsigned` 및 concat/replication 은 이 도메인에 arm 이 없어 여전히 loud, 모듈 스코프 상수식(아래 ①)은 미적용.
- **상수함수 DEFAULT 인자식은 폭 검사 밖**(§4.5.229 재리뷰 발굴·no-oracle 성격): `function int f(input int a = 4'd15+4'd15)` 를 인자 없이 부르면 vita 30 / iverilog 14(iverilog 는 default 를 self-determined 로 본다). 명시 인자 쪽은 반대로 **과잉거부**(iverilog 는 formal 폭으로 넓혀 30 이 정답). §13.5.3 해석이 갈리므로 **두 경로를 의도적으로 대칭화**하는 별도 판단이 필요 — 지금은 비대칭.
- **🔴 package-scope `parameter real` 이 정수 나눗셈**(pre-existing·differential 발견·"미라우팅"보다 넓음): `pk::PR/2`(PR=3) 가 iverilog `1.5000` 인데 vita `1.0000`. module-scope 쌍둥이는 정상. 비-정수 package real 은 loud.
- ~~🔴 fork arm 이 부른 SUSPENDABLE task 미재개~~ **RESOLVED**(§4.5.228) — 근인은 스케줄러 rework 가 아니라 **bb 번호공간 충돌** 한 곳(`FrameRec::is_arm`). `join_none`+`wait fork`·`join_any` 생존 arm 도 같이 고쳐졌다.
- ~~음수 하한 unpacked 배열 원소 누락~~ **RESOLVED**(§4.5.228) — `foreach` 만 쓰는 형태는 **silent** 였다(진단 0건). `lo` 를 i64 로. 부수로 `$bits(a[0])` 오프바이원과 `$readmem` 비-0 base 도 수정.
- **PART select 가 상수 바운드를 unsigned 로 접는다**(§4.5.228 발굴·현재 loud): `logic [3:-2] x; x[1:-2]` — `-2` 가 `const_eval_u32` 에서 0xFFFFFFFE 로 읽혀 방향 검사에 걸린다. §4.5.229 는 이 wrapping 을 **의도적으로 보존**했고(그 값으로 폭을 계산하다 u32 오버플로 패닉이 났던 것을 `folded_part_width` 의 `MAX_NET_WIDTH` 상한으로 막았다), 음수 바운드의 정식 지원은 별도 슬라이스로 남는다.
- **`int'(<real param>)` 를 바운드로 쓰면 조용히 1비트**(§4.5.229 실측·pre-existing·PRE==POST): `parameter real R=3.5; v[int'(R):0]` = iverilog `0d`(=`v[4:0]`) / vita 1비트. `int'(real)` 은 합법적 정수 바운드지만 상수 도메인에 real 산술이 없어 접히지 않는다 — 선행 = §0 T2 `real` const-fold.
- **`$urandom_range(R,0)` 가 범위 1 로 붕괴**(soundness S5): `bits(3.0)` 의 하위 32비트가 0. generic SysCall 인자 경로라 위치별 게이트 필요.
- **파라미터 구조적 지연이 조용히 무시됨(pre-existing·PRE==POST)**: `assign #P y = x;` · `wire #P y = x;` · `and #P g(o,a,b);` 가 P 가 **정수 param 이어도** 지연을 무시(리터럴 `#3` 은 동작). §4.5.221 이 `parameter real` 을 지원하면서 **클럭 주기/지연 관용구가 이 경로의 주 사용처**가 되어 도달성이 크게 넓어졌다 — 우선순위 상향 후보.
- **real→`input int` formal 미강제(pre-existing·PRE==POST)**: `f(2.4)` → vita `24`, iverilog `20`.

> 발굴 경위·재현·범위 상세는 ARCHIVE의 해당 §4.5.x 참조.

**✅ RESOLVED (silent-wrong → loud / correct-support · 잔여만 기재):**

- **real 값 생산자 4종이 index/size gate 全 우회** — **RESOLVED**(§4.5.221 5·6R·상세=ARCHIVE). `atoreal()`·dyn `real d[]` 원소·`ArrSum/ArrProduct`·real 반환 FUNCTION. **잔여 loud**: 계층/package-scoped callee 의 real 반환(다른 테이블로 resolve·보수적) · 바운드가 **산술식**(`v[R+2:R]`·iverilog 도 거부) · `$clog2(<real>)`(iverilog 는 3).
- ~~`foreach` over dynamic array/queue/assoc inside a FUNCTION or SU~~ **RESOLVED**(§4.5.175/§4.5.176·상세=ARCHIVE) — 잔여 loud**: key가 non-local인 direct `st=aa.first(module_net)`(module-net write=`&mut` 필요→fatal). 신규 `frame_foreach_dynamic.rs`×10.

**🔴 DEEP-defer (전용 인프라 필요):**

- `%c`/`%s` high-byte(128-255) UTF-8 remangle — output-pipeline 전체 byte-clean 필요(diag `RtlText`→`Vec<u8>`·~8 test sink·CLI·OBS). §4.5.119/128 발굴.
- derived-localparam self-width (`localparam P=A+B`·`$bits(Q)`=32 등 expression-init 폭) — `param_meta` pollution+allow-list 인프라. §4.5.124 blocker.
- top-level(`$unit`) typedef ② — flat map이 §26.3 scope-precedence 미모델(wildcard-import가 same-name `$unit` typedef 미shadow). 2회 revert. §4.5.104.
- enclosing-const-over-inner-block-local resolution(§6.21 역행) · static-coalesce-onto-automatic. §4.5.108 발굴.
- packed-WIDTH sibling-block 멤버 coalesce(SW2) — read-gate 불가(SOMETIMES-correct)·per-block-scope width 필요. §4.5.98 재분류.

**중형 (오라클 확보 시 착수 후보):**

- dual-wildcard import type ambiguity(둘 다 silent·loud화 후보). §4.5.104.
- mixed-sign enum 산술 · `enum bit` 2-state-base X-leak(일반 enum-base-kind 갭). §4.5.109/110.
- `$signed`-in-wider-sum sign-loss · size-cast fn-call operand sign-loss `4'(f(15))`. §4.5.111/112.
- packed bit/part-select replication count · hierarchical `{s.CNT[0]{v}}` count silent-0. §4.5.109/121 잔여.
- inline-local/generate/task-local param sub-select(`param_range`=module-scope only) · net-vs-param VALUE precedence · gen-scope param value truncation. §4.5.118 잔여.
- 2D-packed iface member element/outer part-select · scalar iface member over-acceptance. §4.5.116 잔여.
- non-uniform `$dist_*` libm draw 발산 · chi_square/t seed LCG. §4.5.126.
- string-array-elem 전원 concat `{s[0],"-",s[1]}` truncate — native-eval static string-width. §4.5.134 발굴.
- `$monitor`에 직접 `$random`/`$urandom` 인자=매 스텝 spurious re-fire(no-oracle: iverilog는 non-simple `$monitor` 인자 자체 거부 "SORRY"·시간함수 3종만 예외). 값 렌더는 정상. §4.5.135 발굴.
- leading-NUL frame string · frame-body 内 SYS-READ(assignment form도). §C/§4.5.122/124. (runtime `$clog2(real)` f64 misread = §4.5.143서 해결)
- format 출력 잔여(§4.5.144 후·전부 pre-existing): real-const `%s`=vita packed f64 bytes vs iverilog warn+`<%s>` · string-LITERAL embedded-NUL(const_string NUL-리터럴+lexer octal-escape `\000`) · render_template malformed spec(missing-arg·`%`+non-spec char)=vita `x`/literal vs iverilog `<…>`+warn(literal+var 공통) · `%d`/`%0d`-of-non-finite real(inf/nan)=vita `i64::MAX` vs `inf`/`nan`. (숫자-const `%s` NUL→space·`%d`-of-real width=§4.5.144/142 해결)
- inline 함수 잔여 4종: global-reader widening 미수혜 · size-cast `16'(a*b)` context · signed `>>>` unsigned-context · inline-call return 미truncate. §4.5.80 잔여.
- hier `@(*)` sensitivity · hier md-packed-nested part-select. §4.5.115/103 잔여.
- param scalar bit/part-select const-fold **미지원**(silent-wrong·DEEP): `logic [P[5:0]-1:0] x`=range-bound 폭 1 vs iverilog 63(param 값 컨텍스트는 E3009 honest-loud). §4.5.148 naive fold `(v>>i)&mask` 시도→**적대 2렌즈 수렴 발굴**: `[N:0]` descending만 정답·**zero-LSB ascending `[0:N]`**(선언 범위 미정규화→wrong bit)+non-zero-LSB below-LSB index=loud→silent 회귀→revert. 근원=`param_range`가 non-zero-LSB만 추적("absent=descending zero-LSB" 불변식·zero-LSB ascending 미탐·`base_net_ascending` false). fix=全 param 범위(lo/msb/direction) 기록 or `[lo..hi]` membership(param_range 불변식 확장=broad).

- enum label **unfoldable-range signed** 잔여(§4.5.158 minor): `enum logic signed [X-1:0]`처럼 base range가 const-fold 안 되면 `enum_base_width`=None→`base_w` None→라벨 sign이 value-inferred(positive 라벨 unsigned)·§4.5.158은 fold되는 base·base-less만 커버. 극한 엣지(unfoldable enum base). fix=`param_range` 불변식 확장 or value-inferred에 enum sign 전파.
- ~~🔴 size-cast `N'(expr)` context width 미전파 (DEEP·broad)~~ **RESOLVED**(§4.5.212·상세=ARCHIVE) — 잔여 residual**: param/call leaf(`8'(P*a)`)는 fallback→여전히 self-width(follow-on).
- **`$bits(md-array ROW)` partial-index sub-array** = next-pow2 오류(`byte m[2][3]; $bits(m[0])`=vita 32 vs iverilog 24·`int`=128 vs 96) — logic 배열도 동일(atom-independent)·element `m[i][j]`는 정상(§4.5.155). runtime 경로만(const-context는 §4.5.155가 부수 교정). from_prescan partial-index 미처리→from_table next-pow2. pre-existing·§4.5.155 발굴.
- **class value-param 타입 폐기(no-oracle·§4.5.159 발굴)**: `parse_class_param_list`이 `ClassParam{name,default}`만 보유→선언 타입(`#(parameter byte N=300)`의 byte/[3:0])을 파싱만 하고 폐기→narrow 타입 truncation 미적용(`N`=300 vs 44 예상). **oracle 없음**(iverilog가 typed class value-param 미지원=mutual loud on `[3:0]` form). 헤더 param(§4.5.159) 상속과 별개 경로·pre-existing. fix=`ClassParam`에 type 필드 추가(AST·schema re-pin)+elaborate coercion(전용 슬라이스·no-oracle→hand-IEEE).
- **untyped param 값-결정 타입 잔여(§4.5.161 LITERAL·§4.5.162 IDENT/EXPRESSION·§4.5.163 PKG-scoped 해소·이하 residual)**: (a) **`time` param value-inferred**: `localparam time C=<expr>`가 value-inferred(negative value→32-bit signed·`$bits(time-alias)`=32 vs net 64)—time은 value-determined에서 의도적 제외라 declared 64-unsigned 미적용. fix=time param에 (64,unsigned) meta 배선. (b) **expr self-determined WIDTH**(`8'hFF+8'hFF` 등 sub-expr truncation·§2 size-cast DEEP 동근·sign만 정확·width=`min_signed_bits(folded)` 근사). (c) **interface-member** positive param(`i.A<i.B`)=unsigned(module-hier/pkg 정상). oracle=iverilog ✓. 전부 pre-existing·미악화 확인.

- ~~🔴 inner NET 이 outer PARAM 을 shadow 못 함~~ **RESOLVED**(§4.5.246) — 근인은 `lower_expr` 의 fall-through 가 `lookup_scoped`(params 전용 walk)를 불러 방금 도출한 innermost 키를 무시한 것. **잔여**: ① **모듈 레벨 프로세스**의 블록 로컬(`initial begin int W; …` — flatten 키가 param 과 같아 술어가 구분 불가·iverilog 9/vita 4) ② enum-label·package 변수 clobber(형제 항목). §4.5.247 이 generate 스코프 누출 회귀를 수정했다.
- **block-local이 imported package 변수를 clobber(§4.5.218 재감사 발굴·오라클 有)**: `package pk; int pvar=33;` + `import pk::*` + `initial begin int pvar; pvar=7; end` → `pk::pvar`이 vita **7** vs iverilog 33(IEEE §26.3: 로컬이 import를 shadow·패키지 변수 불변). alias net이 `symbols`에 있어 `existing=Some`이지만 `int` 패키지 변수는 dyn/string 3-clause guard를 전부 통과해 coalesce. function-local은 안전(`$func$` scoping).
- **static 블록 로컬의 초기화자가 같은/바깥 블록의 `automatic` 형제를 읽으면 loud(§4.5.248 상승)**: static init 은 t0, automatic 은 블록 진입 초기화라 값이 없고 output/inout copy-out 은 진입 초기화에 덮인다. **PRE 도 silent 였다**(exit 0 에 copy-out 前 값) — fork 제한을 걷으며 드러나 loud 로 올렸다. 지원 자체는 per-activation 저장이 필요해 여전히 비목표.
- **same-name dyn 로컬 잔여 2형(§4.5.249)**: ① static + **초기화자**(`$blk$` 경로가 decl-init 수집기를 건너뛰어 `size()==0` 을 실측 — 그래서 제외하고 loud 유지) ② 스칼라 `string`(자체 coalesce 가드가 따로 있고 같은 init-drop 위험). 둘 다 `$blk$` 경로에 decl-init 수집을 붙이면 열린다.
- **block-local 잔여 shadow 2형(§4.5.218·PRE==POST)**: block-local **scalar vector**의 이름을 block이 index-select(`logic [7:0] sa; sa[0]`)=vita 0 vs iverilog 1 · named-block array 1형. **이름 충돌만으로 gate하면 byte-correct 설계 11건이 false-reject**되므로(§4.5.218 실측) per-shape hazard 모델이 필요.
- **scalar-string 같은-family 잔여(§4.5.217 differential 발굴)**: `expr_is_string_ast`에 **`Ternary` arm 부재**→`{(c?a:b),"!"}`가 바이트 drop(scalar·element 공통·**no-oracle**[iverilog assert]·hand-IEEE) · `{a, 8'h00}`/X-Z 바이트가 `0x20`으로 치환되고 길이에 계상. **정정**: 같이 기록됐던 "`always @(*)` string concat 조용히 drop"은 **오진**—vita가 `[abcd]`로 정확하고 iverilog가 빈 문자열이며, 명시 `@(a,b)`는 vita가 정직하게 loud.

**문서화된 divergence (수정 비대상·핀됨):**

- 크로스스코프 t0 decl-init race(양쪽 §6.8 합법·self-consistent) · 런타임 구성 `-0.0` 표시 · iverilog 자인 결함들(expression-force "evaluated once" 등).

## 3. Loud→supported 후보 (현재 전부 loud=안전 · additive)

**~~frame-body validator over-scan~~ RESOLVED §4.5.172** (pre-existing false-REJECT · §4.5.171 적대 agent 발굴 · V5 무관): `classify_frame_body`의 linear `block_base..func_blocks.len()` 스캔이 **POST-PASS**(`resolve_frame_task_rejects`·전 func lower 후 subset task 검증)서 `func_blocks.len()`=**전체** 끝이라 **뒤에 정의된 func 블록까지 over-scan**→그들의 (합법적으로) out-of-frame인 output-formal write를 자기 것으로 오판→subset task를 E3009("assignment to a net outside the function")로 false-reject(iverilog는 accept). **fix = reachable-block CFG walk**: entry(`self.funcs[fid].entry`)서 자기 CFG 엣지(`Goto`/`Branch`/`Call`→`ret_bb`[callee entry 아님]/`Delay`·`Wait`→`resume`)만 순회→다른 func·dead 블록 미방문. **correct-or-loud**: 미방문 블록은 타 func이거나 dead(실행 안 됨)이므로 verdict는 false-reject만 DROP(silent-wrong 불가). `frame_task_pending` 튜플서 dead `base` 필드 제거. 적대 differential 全 MATCH(nested call·if/case/for·suspendable-mixed·5-task interleave·still-loud=모듈 net write). 신규 `frame_subset_overscan.rs`×4. 상세=ARCHIVE §4.5.172.

**string/heap:**

- ~~frame string LOCAL 대입(E3018→heap slot)~~ **RESOLVED**(§4.5.167·상세=ARCHIVE) — 잔여 loud**: static(non-`automatic`) task inline string local(`hoist_inline_task_locals`:18119)=Wire→E3018(inline 경로=frame-slot 아님·순진 String化시 str_bytes twin 미적용→silent 회귀 위험·별개 inline-string-storage 슬라이스). substr-actual `s[i]` · string part-select `s[i:j]`.
- dyn string 요소 method(`s[i].len()`) · whole-element read as value(`x=arr[i]`) · record array-of-record.
- queue/assoc의 string·real 요소 · string queue · block-local queue decl.

**함수/패키지/formal:**

- ~~control-flow pkg fn~~ · ~~pkg `function string`~~ = **둘 다 이미 동작**(2026-07-25 재그라운딩: `9` / `hi`). 잔여 = pkg TASK statement call뿐. §4.5.111.
- array formal 재전달(nested/recursion) · non-zero-LSB 원소 · 2-D/non-zero-base/signed/task array formal. §4.5.110 slice 밖.
- 음수-LSB 멤버 sub-select 정식 지원(§4.5.114). md-packed nested part-select WRITE `x[j][m:l]`/`arr[i][j][m:l]`=**§4.5.145 지원**(descending zero-lsb leaf·const packed idx); fail-closed 잔여(전부 loud·honest)=ascending/non-zero-lsb leaf·**genvar-index**(`x[g][m:l]`=const-fold 안 됨·over-reject)·const-OOB packed idx=silent no-op(read path 공유·값 무손상).
- method/ctor NAME-default class-scope 해석(§4.5.90) · G4 string-return frame call(§4.5.129).
- **round-14 리포트 잔여 (전부 loud·오라클=iverilog ✓·§4.5.167서 스코프)**:
  - ~~V3/V4 (task body 内 timing/suspend·최우선)~~ **RESOLVED**(§4.5.168/§4.5.169·상세=ARCHIVE) — 잔여 loud(§2 defer 후보·전부 correct-or-loud)**: 용·`reserve_frame_local_decl` 4-site 통일·엔진 frame_local=net-range라 자동 격리·적대 differential common 全 MATCH·automatic-array 격리/subroutine port는 vita>iverilog) · `wait(<frame-local>)`(level-wait re-eval window 미복원)·`repeat(<non-const>) @`(hidden counter SHARED net·동시활성화 오염)·NBA-to-frame-local(illegal)·`fork`/`disable fork`/`wait fork`-in-task(fork-family 미지원). 각 근본해결=per-activation repeat counter·in-frame fork machinery(별개 슬라이스). **§4.5.169 잔여 loud(safe gap)**: frame-local array의 multi-dim·non-zero-based·non-simple-element·whole-copy(`b=a`)·`foreach`·NBA-elem·`'{…}` init.
  - ~~V2A (task input dyn-array formal)~~ **RESOLVED**(§4.5.170·상세=ARCHIVE) — 잔여 loud(V5 의존)**: **automatic/recursive task**의 dyn-array formal은 frame formal이 scalar slot이라 handle-in-slot(V5) 필요→loud-defer. write-to-formal/sign-mismatch/non-bare/queue actual=correct-or-loud.
  - ~~V5 (frame dyn-array local `new[]`)~~ **RESOLVED**(§4.5.171·상세=ARCHIVE) — 잔여 loud(follow-on)**: (a) **FUNCTION** dyn-array local(`&self` 동기 `run_frame_call`/`run_task`가 `new[]`=`&mut` heap 실행 불가·구조적) (b) **recursion/concurrent**(per-activation heap stash 필요=진짜 격리) (c) multi-dim/packed/non-bit-vector element.
  - **잔여 loud(follow-on)**: (a) FUNCTION dyn-array local (b) recursion/concurrent(per-activation heap stash) (c) multi-dim/packed/non-bit-vector element.

**~~inline(static-task) 경로 `foreach`-on-dyn-formal~~ RESOLVED §4.5.174** (§4.5.170 gap·§4.5.173 발굴): static(non-`automatic`) task/function의 `input b[]` formal에 `foreach(b[i])`가 E3009("first/next/last/prev are only supported as the DIRECT rhs...")였음. **원인**: 파서가 `foreach(b[i])`를 `__st=b.first/next(__i)`로 uniform desugar→elaborator가 array KIND로 재작성하는데 dyn/queue dense-walk dispatch가 `dyn_handle`(=`lookup_net_scoped`)로 resolve→`dyn_subst` formal alias(read-only input dyn-array formal은 real net 아님) 못 봄→`b.first(__i)`가 generic method 경로로 falling through→E3009. **fix**: `foreach`는 배열 READ이므로 dispatch(dispatch gate + resolution 2 site)를 `dyn_handle_read`(alias 우선 consult)로 교체→formal이 module dyn-array와 **동일한 dense walk**로 라우팅. `dyn_subst`는 inline dyn-formal body 밖에서 empty→**모든 다른 array(fixed/module dyn/queue/assoc) byte-identical**. 적대 differential 全 MATCH(index+element·signed·two-foreach·module dyn/fixed/queue regression). **잔여 loud(별개 follow-on)**: FUNCTION+`foreach`(control flow라 R2-inline 불가→framed→framed function dyn formal 미지원)는 여전히 loud=function-frame dyn-formal 슬라이스 소관.
- ~~round-16 리포트(executor-bound)~~ **RESOLVED**(§4.5.194·상세=ARCHIVE) — **잔여 loud(follow-on·correct-or-loud)**: ① dyn local/formal을 쓰는 **recursion**(per-activation heap stash 필요) ② **string/real/class-handle** element의 dyn formal(input 포함 제외) ③ unwritten output formal = IEEE §13.5.2 empty copy-out(iverilog는 by-ref로 caller 값 유지=非준수).

**~~frame-body dyn-formal FUNCTION call~~ RESOLVED §4.5.195 (round-17 C)**: `s=sum(b)` in a TASK body(리포트 케이스)가 E3009였음. §4.5.194 RefCell dyn_heap이 `&self` executor의 snapshot-marker heap-copy를 가능케 함→elaborate gate `in_frame_body` 제거·`classify_frame_body`가 전용 `dyn_formal_marker_stmts` set의 marker만 허용(§7.10 whole-copy는 frame body서 loud 유지)·`run_frame_call`/`run_task`에 marker arm(`frame_dyn_copy_out`). TASK-body direct/buried·function-local-arg·module-process 全 iverilog MATCH·recursion=F4004 loud. **잔여 loud(correct-or-loud follow-on)**: FUNCTION이 자기 dyn-formal을 다른 함수로 **재전달**(`fsum(c)` where c=이 함수의 formal)—framed function formal이 heap-resident라 `dyn_array_actual_net`이 re-forward source로 못 resolve(기존 "array formal 재전달" 갭). `frame_body_dyn_formal_call.rs`×7.

**round-17 D-가족 (리포트 §5 minor 잔여 — §4.5.196서 tractable 4항목 RESOLVED):**

- ~~계층 태스크호출 `u1.tk(x)` (TASK·L)~~ **RESOLVED**(§4.5.196/§4.5.197/§4.5.198·상세=ARCHIVE) — 잔여 loud(correct-or-loud)**: output/inout/array/string formal[cross-boundary copy-out·별개]·STATIC(non-automatic) task hier-call[non-framed→force-frame이 local caller 회귀 위험=large·**정공법=frame⊂inline 동등화**·§4.5.198 step 1 착수]·~~module ARRAY-element write in framed task~~[**§4.5.198 RESOLVED**·`word.is_none()` 게이트 제거→suspendable `&mut` array write]·nested-in-frame-body hier enable[`task_calls_func` target transitivity]. 신규 `hier_task_call.rs`×14·`frame_task_module_write.rs`×6·기존 `frame_subset_overscan.rs` 1 flip.

- ~~frame↔inline parity · 배열 formal shape · hier-task 계열~~ **RESOLVED**(§4.5.198~208·상세=ARCHIVE) — frame ⊇ inline 완성, 배열 formal은 input/output/inout × 1-D/multi-dim × zero-based-any-direction + non-zero-base-ascending × automatic/static/function 전부 커버. **잔여 loud**: ① non-zero-base **descending** 배열 formal ② **hier-task OUTPUT/INOUT 배열 formal**(deferred copy-out·hard) ③ frame-formal 배열을 nested hier로 forward ④ generate-block 内 hier task call(pre-scan이 top-level proc block만 봄) ⑤ hier-task의 string formal.
- **잔여 follow-on (전부 loud·correct-or-loud)**: **function이 자기 dyn-formal 재전달**(`return sum(c)`·framed callee·risky: mutual-recursion soundness hole→frame-route 시 신중 guard 필요·`nested_in_frame_body_loud` 유지) · **block-local automatic lifetime**(loop-body `automatic int j=k*10` 등·per-activation storage 필요=deep block-local-flatten 인프라·iverilog도 "Overriding default variable lifetime" 거부) · **chained-method** `q.min()[0]`(vague·iverilog syntax-error·no-oracle). **이미 동작(vita>iverilog)**: output-fn-in-`while`(D4)·task OUTPUT array(D5b·§4.5.193).

**소형 큐:**

- **`$typename` 의 enum / packed struct 렌더**(§4.5.239 발굴·무오라클): base 타입으로 나온다(`logic[1:0]`·`logic[3:0]`; IEEE §20.6.1 은 `enum{...}`·`struct packed{...}`). 타입 이름 렌더링 한정이라 값 영향 없음 — 현행은 `typename_pins.rs` 가 핀.

- **`$value$plusargs` expression 문맥 = 소형 후보 아님(§4.5.240 정정)**: 관용적 배치 **둘 다 이미 동작**한다 — `ok = $value$plusargs(…)` 와 **`if ($value$plusargs(…))`**(iverilog 일치, §4.5.240 서 핀). 남은 loud 는 `$display("%0d", $value$plusargs(…))` 류뿐이고, 이는 **side-effect sysfunc 패밀리 전체의 설계**다(seeded `$random`·`$fopen`·`$sformatf`·fd-advancing 파일읽기 — 전부 single-eval 보장을 위해 statement-form 으로 lower). 임의 expression 위치는 desugar 할 statement 가 없어 loud 가 정답이며, 넓히려면 **패밀리 전체의 desugar 확장**이 선행(소형 아님).
- **파일 위치 함수군 + `$sscanf` = honest-loud**(§4.5.238 실측): `$ftell`/`$sscanf` 등은 E3009 "unsupported system function in expression", `$fseek` 는 **W3056 warn+skip**. iverilog 는 전부 동작(`A=6 B=0 C=6 D=0`, `$sscanf`→`2 12 34`). **silent-wrong 아님** — ② loud→supported 후보. **스캔 소진**(§4.5.239): `$typename` 은 iverilog 미구현인데 vita 정확 → 핀. `%u`/`%z` 는 양쪽 다 문서화된 선택(vita 무출력·iverilog raw 바이트), `%l` 은 cosmetic.

- **`%p` 의 string / unpacked-struct 렌더**(§4.5.236 발굴·무오라클): string 이 packed 바이트 값으로(`"hi"`→26729), unpacked struct 가 필드 연접 정수로 나온다(IEEE §21.2.1.7 은 `"hi"`·`'{x:7, y:-2}`). 렌더러의 `Value` 에 `is_real` 만 있고 string/struct 마커가 없어 **타입 정보 전달**이 선행. real 은 §4.5.236 에서 해소.

- **string byte select on a PARENTHESIZED base가 조용히 0**(§4.5.220 재감사 발굴·pre-existing·scalar/fixed/dyn 全 형태): `(p)[0]`=0 vs `p[0]`=119. **paren-select 일반 갭이 아님** — `logic[7:0] v; (v)[0]`·`(v)[3:0]`은 bare 형태와 동일하게 정확. string 전용이며 근인은 `string_index_read`의 base gate가 `Ident|BitSelect`만 matches!하고 `Paren`을 unwrap 안 함(반면 `expr_is_string_ast`에는 `Paren` arm이 있음)→byte-select 경로에 도달 못 하고 width-0 handle의 packed bit-select로 낙하. **§4.5.220이 고친 것과 동일 실패 클래스**(gate가 자기 술어를 under-approximate)·한 gate 옆. iverilog는 구문 자체 거부(오라클 無).
- **scalar `real`의 part-select write가 조용한 no-op**(§4.5.220 재감사 발굴·pre-existing): `real x; x[3:0] = 4'hF`→값 불변·무진단. iverilog는 거부("can not select part of real"). §4.5.220이 dyn `real` ELEMENT(`r[0][3:0]`)를 loud화했으므로 **이제 scalar 쪽이 뒤처진 비대칭**(방향이 string 케이스와 반대·회귀 아님).
- **array reduction method가 var-init initializer에서 loud**(§4.5.219 재감사 발굴·pre-existing): `string s = $sformatf("%0d", arr.sum());` 및 배열 원소 형태 모두 E3009 "unsupported hierarchical function call arr.sum" — t0 pre-sweep 경로의 갭이며 scalar/array 동일(=게이트 문제 아님). `q.size()`·`.len()`·`.substr()`·`.name()`은 동작.
- **string-array 잔여 → §0 승격 큐 T1로 이관(2026-07-25)**: FIXED string array decl-init(`string s[2]='{"a","b"}`·module/block 양쪽 loud·iverilog ✓·§4.5.183 기록 항목) · fixed array **runtime index**/`foreach`(dyn 배열은 이미 동작→fixed만 element-net 표현 때문에 const-index 전용) · `string q[$]`(queue of string·iverilog ✓) · multi-dim `string s[2][2]`(iverilog ✓) · hierarchical `u.s[0]` · **frame-local**(task/function body) string array(static task=E3018·function/automatic=E3009). dyn element의 byte select `d[0][0]`(no-oracle)도 잔여.

- ~~gen/iface string decl-init~~ **RESOLVED**(§4.5.228). 잔여 = generate-case 스코프 이름 `gcase[0].x` · 인터페이스 queue 원소의 **계층 read**(`u.q[0]`).
- SYS-READ hier-element dest · hier-write sentinel panic→loud · generate-내 `import` · package 자기-func init(㉽). explicit `import p::t`(TYPE)=**§4.5.148 지원**.
- `$fmonitor`/`$fstrobe`(파일 strobe/monitor) — 현재 W3056 skip=**파일출력 silent drop**(non-silent·warned). 지원=**format bump 필요**(`SysTaskId` 변종 ① or 직렬화 사이드카 ②·staged 파리티): `FmtCapture`에 `fd:Option<u32>` 추가(engine-local)+strobe drain을 `file_write` 라우팅·전용 슬라이스. STDIN read(결정성 설계 필요).
- compound-const `==?` fold=**§4.5.146 지원**(sized 패턴)·잔여 fail-closed loud=unsized x/z 패턴(`'hx` self-width truncation)·negative-signed LHS·non-literal RHS. param override 비상수(W3056→error) · longint MIN fold(package) · loud-message 품질 2건(`[bit]` 캐스케이드·typedef-키 메시지).
- `case (x) inside {…}`(§12.5.4 wildcard case)=vita E2002 parse-reject(loud)·③ 후보(no-oracle: iverilog 13.0 `case inside`/`inside` op/array reduction method 全 거부→hand-IEEE `==?`+내부차분). `inside` operator는 지원(== 시맨틱·§11.4.13). based-literal 내 whitespace(`64'sh FFFF`)=vita lexer reject(loud) vs iverilog 허용(minor·§4.5.147 발굴).
- ~~enum label 범위검증 부재~~ **RESOLVED**(§4.5.165·상세=ARCHIVE) — 잔여(fail-open·§2 invalid-program 한정)**: sized/based-literal label(`{A=8'hFF}`)·param-width base(`[N-1:0]`)의 out-of-range는 `const_lit` 미fold(decimal-only)라 skip=silent-truncate 잔존(iverilog reject하나 유효 프로그램 무영향·fail-open>over-reject·const_lit 확장 or elaborate-time 검사=별개 슬라이스).
- **sized-literal enum label → enum-method loud**(§4.5.164 발굴·pre-existing): `enum bit[3:0] {A=8'hFF}`처럼 label이 non-foldable(sized-literal/식)이면 `enum_defs` 미등록 → `.first`/`.next`/`.name` 全 enum-method가 E3010/E3009 loud(honest·유효값엔 무영향). `const_lit`이 unsized-decimal만 fold. diagnostic 품질 minor(`.name` 메시지="hierarchical function call deferred"·오도). fix=enum label const-fold 확장 or label-site 진단 개선. 부수: function-port receiver `.name`/`.first`=E3010(`var_enum`이 tf-port 미bind·enum-method-family 공통)·chained `x.name().len()`·`arr.min()[0]`=iverilog도 거부(no-oracle).
- **partial-timescale 정책 진단**(`--timescale-policy`·`W-PARSE-TIMESCALE-PARTIAL`/`E-PP-TIMESCALE-PARTIAL`): 일부 모듈만 `` `timescale `` 선언 시 현재 무진단 1ns/1ns 디폴트(전무 케이스만 W1017). doc-08 §15 설계는 문서화됨·`rt.default_used` 신호 이미 존재 — 배선만 필요. §4.5.151 발굴.

**외부 round-20 리포트(2026-07-27) = 12 가족 중 10 RESOLVED**(§4.5.229 part-select 바운드 · §4.5.234 enum formal `.name()` · §4.5.248 8 가족 · §4.5.249 §6 진단 위치). **잔여 = §4.11 의 미격리 79 건** — 리포트 자신의 16-블록 충실 복제본도 재현 못 했고 이 체크아웃에서는 볼 수 없다. §4.5.249 의 file:line 이 리포터가 좁힐 수단이다.

**외부 리포트 잔여 (§6-2 → ARCHIVE · 전부 no-oracle 또는 docs):**

- EXT2-A2c: packed multi-dim param `localparam logic[1:0][7:0] PK=…`(외부 0회 사용·hand-IEEE+내부차분).
- EXT2-NAP: named assignment pattern `'{k:v}`(외부 0회).
- EXT2-DOC: 문서 stale(CLI-ref·lang-ref·system-tasks·explain — 외부 2회 보고).

**deep 잔여(저우선):**

- t0 race 그라운딩(계단식 CA 체인) · `@(*)` decl-init wake · runtime `==?` pattern.
- inline body NON-fill context-width · modport 방향 강제 · force part-select · assoc 배열-key/clocking 배열-output word0.
- ~~음수 range bound~~ **RESOLVED**(§4.5.228) — net·multi-packed inner·unpacked·VCD 범위. 정규화 저장(`[w-1:0]`) + 선언 바운드 사이드맵, 선택 정규화와 폭을 **함께** 켜는 opt-in. `[W-1:0]`-with-W==0 param underflow 는 graceful width-1 유지(test `v3_12`). **잔여**: PART select(§2) · 포트/formal(의도적 비대칭).
- **VCD 잔여 fidelity**(§4.5.138 range fix 후·전부 **cosmetic·decode 동일**·§4.5.139서 VALUE 검증 완료: x/z·real·wide·readmem·format 全 decoded waveform iverilog 일치). 남은 encoding 차이: ① value 미압축=vita full-width(`bxxxxxxxx`) vs iverilog leading-redundant strip(`bx`·`b0`)—decode 동일·큰 golden churn ② t=0 초기덤프 구조=vita `$dumpvars`에 pre-assign X + `#0` change vs iverilog settled값—final 동일 ③ var-type=logic 절차구동시 vita `wire` vs iverilog `reg`(연속구동=both wire·usage 의존이라 non-trivial)·`int`=`reg` vs `integer` ④ real size `64` vs `1` ⑤ `parameter` 미덤프. + 근본: elaborate packed-md NetVar.lsb stale(lib.rs 8435/7862·VCD helper서 flat fallback 우회).
- **real const-fold 전면 미지원**(§4.5.141 발굴): `localparam/parameter real` = `2.0+3.0`·`*`·`/`·`-`·`**` 全 E3009 "not foldable"(iverilog=folds). `localparam=$clog2(real-lit)`도 동근(const_eval_in_scope=i64-only·real arg→None loud·§4.5.143 런타임은 해결). 런타임 real 산술은 정상(§4.5.141서 `**`도 지원)·const 경로만 uniformly loud. const_eval_in_scope에 real f64 arithmetic 추가 필요(broad·non-silent).
- **X-bearing integral→real 변환 divergence**(§4.5.141 발굴): vita=whole X값→`0.0`(`real_arg`=`to_i128_signed().unwrap_or(0)`) vs iverilog=per-bit X→0(예 `4'bxx01`→1). `$itor`/`$sqrt`/`$pow`/real-`**` 공통·pre-existing. non-silent 아니지만 divergent(impl-defined X→real).
- **width>128 정수→real 변환**=여전히 `0.0`(§4.5.151서 `to_i128_signed`를 128-bit lane까지 확장 — 65..=128 signed/unsigned는 수정 완료·>128만 잔여). 초희귀(129-bit+ 값의 real 대입)·word-grid f64 근사 필요.
- **x/z-fill const param LHS→0**(§4.5.146 발굴): `localparam logic [W] P = 'x`=const_eval가 `fill_to_i64`/`fill_literal_const`로 0 bind(x 소실)→**全 const 연산자 상속**(`P==0`·`P+1`·`P ==? pat` 4-state 결과 divergent). upstream param binding 근원·contrived(all-x const 선언)·broad. §4.5.146 `==?` fold는 sized 패턴만이라 무영향(a=int).

## 4. SVA / 검증 honest-loud 잔여

- empty-match `##0`/unbounded `##[m:$]` 융합(§16.9.2.1 불연속·오라클 부재).
- N2c full sequence local var(중첩 attempt 각자 데이터=L급; 단일-capture ✅).
- later-antecedent read · outer-`|=>` prop-ref skew 고급형(2-cycle·중첩·cross-clock).
- SVA-QUAD collapse default-flip(`VITA_SVA_COLLAPSE` opt-in 상태 — full-VCD 골든 audit 선행).
- N4 clocking 잔여: non-`#1step` skew·INOUT·multi-event-list clock·non-net bind·hier input drive·cross-hier `@(inst.cb)`.
- class: down-cast `Derived'(base)`($cast 런타임 타입가드 선행) · real→longint cast · base-shadow 명시 접근 `Base'(d).v` · cast-as-receiver `(B'(d)).foo()`.

## 5. perf / 하드닝 잔여 (전부 보류 판정 — 트리거 시만)

- SVA-QUAD default-flip = §4 항목과 동일(full-VCD audit 선행).
- FMT-CACHE part b(render_template pre-segment) · GEN-3X-STR part a(unroll plan 캐시=byte-identity 위험>이득) — 저ROI 보류.
- QUEUE-MID-ON: 스펙 내재 O(n)(iverilog 동일) — 영구 비권장·monitor-only.
- 백로그 원문·완료 32건 = ARCHIVE §5.

## 6. G2 — AI-Agent 친화 OBS 트랙 (SPEC=[preview/19](preview/19-ai-agent-observability.md))

> 완료: OBS-0 스펙 · OBS-1a run.json+results.jsonl(§4.5.73) · OBS-1b coverage.json(§4.5.99) · OBS-2 v1 trace.jsonl(§4.5.100) · OBS-3 stage.jsonl(§4.5.101) · OBS-S0 설계구조 export `--hier-tree`/`--inst-paths`(2026-07-13). teeth=3-way 내부 차분(JSONL≡VCD≡`$display`)+결정성 골든. 틀린 로그=silent-wrong과 동급. 값 인코딩 실태(§4.5.151 문서 정합)=trace `old`/`new`=full-width 4-state binary·stage `vals[]`=`%0d` decimal — doc-19 §3 pin 4에 기록.

| 단계 | 산출물 | 공수 |
|---|---|---|
| OBS-2 잔여 | sva.jsonl(R-L6·SVA property명+support-cone v0) · per-element array probe · class/event probe(no-oracle) | M |
| OBS-1 잔여 | staged/vrun obs(.velab source-identity) · compile-fail manifest · `--seed` | S-M |
| R-L4 | 로그 채널 분리 | M |
| OBS-4 | `vrun --control stdio` JSON-RPC(peek/poke/step/run_until)+poke 저널 replay | L |
| OBS-5 | snapshot/restore/rewind(엔진 상태 postcard 직렬화) | L-XL |
| OBS-6 | X-origin·region-annotated events·정적 backward cone | L+ |

- 비목표: FSDB/UCDB·SQLite 내장·waveform GUI·UVM 연동. VCD는 사람용 유지.

## 7. 조건부 / 장기 (재진입 트리거 충족 시에만 승격 · 정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | ① cycle-based 컴파일드(Verilator급) ② PDES BSP 병렬(Amdahl 상한 T4≈2.5x) ③ native-eval 잔여 lane(signed>64·>128bit·sysfunc·real) | ① 대형 RTL 실수요 ② 지속 W≥64+grain≥200ns ③ 저ROI 상시 defer |
| VHDL | VHDL 프론트엔드(9-value std_logic 매핑·별도 파서·GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원** — `$dumpfile("x.fst")`/`-o x.fst`; known-edge=소형 타임테이블 fst-writer [issue #4] loud 거부, preview/07 참조) |
| MVP-CUT | string concat-nonassign · wildcard assoc `[*]` · package internal-import/scoped-call 잔여 · cross-frame disable | 개별 수요 시 |

## 8. 비계획 (영구 비목표 · gap 아님)

- **DEFPARAM**(IEEE deprecated·`#(.param())`로 충분) · **IMPLICIT-NET**(정책=E3010 명시 에러) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).

## 9. 완료 이력 포인터

- 완료 슬라이스 상세 로그(§4.5.3~§4.5.134)·구 §0~§7 원문 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존).
- 탄 단위 내러티브·방법론 교훈 = [DEVLOG.md](DEVLOG.md)·ARCHIVE §3.
- 외부 호환성 리포트 1·2차 전말(A1~C1·EXT2 체인) = ARCHIVE §6·§6-2 — **잔여는 위 §3 "외부 리포트 잔여" 3건뿐**.
