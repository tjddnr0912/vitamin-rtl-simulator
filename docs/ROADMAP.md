# ROADMAP — 잔여 과제 (vitamin)

> **이 문서 = 전방(남은 것)-전용.** 완료 항목의 상세 로그(§4.5.x)는 [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)에, 옛 §번호(구 §0~§7) 원문은 [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md)에 있다(둘 다 §번호 보존). 이력 내러티브 = [DEVLOG.md](DEVLOG.md), 상위 스냅샷 = [REMAINING_WORK.md](REMAINING_WORK.md), 실행 큐 = `LOOPROMPT.md` NEXT(로컬 dev-meta), SPEC 정본 = `docs/preview/`.
>
> **기준선(2026-07-31)**: format_version **26** · **5009 tests green** · 3-OS CI green · MsgCode **59** · **MSRV 1.85**. 최신 = **§4.5.278**(외부 round-23 — 호출의 **copy-out 목적지**를 분류기가 아예 안 보고 있었다: `Terminator::Call` 은 `{target, ret_bb}` 만 들고 목적지는 call-site 사이드 테이블에 살아서 `Stmt` lvalue 를 걷는 워크에 안 보였다 → 프레임 본문의 bare call 이 모듈 net 에 copy-out 하면 **진단 없이 rc=101** · 그것을 고치자 이웃 loud 4형태가 correct-support 로 · E3009 문구가 **패닉하는 형태를 "동작한다"고 명시**하던 것 + 거부된 terminator 를 전부 "timing control" 로 부르던 것 + 내부 `.expect()` 패닉을 fatal 로 · loud 를 걷자 드러난 `StrPutC` 프레임-로컬 문자열 silent-wrong · §3.2 성능은 **iverilog 가 같은 깊이 스케일링**임을 실측해 리포트의 뿌리 가설을 반증. 3-way 70프로브 회귀 0 · fixed 26 · 신규 테스트 14). 직전 = **§4.5.277**(외부 round-22 — 어떤 실행기가 서브루틴 본문을 돌리는지가 **본문 안의 무관한 `$display("x")` 한 줄**에 달려 있었다. 분류기가 blocking assign 을 **목적지로만** 봐서 효과가 든 rhs 를 못 봤다 · 함수도 같은 뿌리(집합이 주는 건 suspend 가 아니라 `&mut` 실행기) · static task 의 `string` 로컬은 같은 개념의 **세 번째 수집기** · fatal 이 자기 `$finish` 에 져서 안 멈추던 것 · 그리고 §4.5.275/276 이 두 번 되돌린 "이름 없던" 패닉 조건에 이름을 붙였다 = **copy-out 의 모든 목적지가 프레임 창 안인가**. 3-way 전수 회귀 0 · fixed 35 · 신규 테스트 21). 직전 = **§4.5.276**(외부 round-20 — 내가 만든 회귀와 그 밑의 silent-wrong 5건). 직전 = **§4.5.275**(값을 반환하는 output-formal 호출을 **아무 표현식 위치에서나** — 한 shape 서술을 네 워커가 공유 · 조건부 자리는 guard 블록 · 왼쪽 읽기는 pre-call 스냅샷 · 적대 리뷰 2라운드가 잡은 silent-wrong 12 + pre-existing 5, 그중 하나는 엔진 패닉). 직전 = **§4.5.274**(외부 round-19 — 값을 반환하는 호출의 output actual 33/34 · `void'(f(out))` 문장 · named arg 매핑 · silent-wrong 2건: default 인자 스코프 · frame body 안의 파일 읽기). 직전 = **§4.5.273**(외부 round-18 — suspend 하는 callee · struct 멤버 비트 커버리지 · 파서가 떨어뜨린 `automatic`). 직전 = **§4.5.272**(`-v` 유효 invocation echo). 직전 = **§4.5.271**(오라클을 만들다 나온 silent-wrong 2건 — 리시버를 못 보는 참조 워커 · `atoi` 계열이 `strtol` 이었다). 직전 = **§4.5.270**(안 쓴 로컬은 per-entry 저장과 바이트 동일 — 그리고 그걸 열자 §23.9 구멍이 드러났다). 직전 = **§4.5.269**(외부 round-17 §3.1/§3.1b/§3.3 — arm 하나가 없었고, catch-all 하나가 이미 쓴 걸 잊고 있었다). 직전 = **§4.5.268**(외부 round-16 §3.4~§3.7+§4 — 두 단계가 스코프를 다르게 중첩). 직전 = **§4.5.267**(고정 크기 `automatic` unpacked 배열 — per-entry 리셋은 측정으로 기각). 직전 = **§4.5.266**(definite-assignment 이 제어 흐름과 callee 본문을 본다 — 리포트 84건 중 53건). 직전 = **§4.5.265**(초기화자 소유권을 랭크 경로로). 직전 = **§4.5.264**(gen-item 리스트의 맨몸 `begin…end` 도 문법). 직전 = **§4.5.263**(generate REGION 은 스코프가 아니라 문법). 직전 = **§4.5.262**(bind band 를 인스턴스 경계에서 리셋). 직전 = **§4.5.261**(인스턴스 랭크 성분 3개 — root 순서·`bind` 위치·배열 원소). 직전 = **§4.5.260**(인스턴스 랭크를 선언 오프셋으로 — 재리뷰가 잡은 F4 맞바꿈). 직전 = **§4.5.259**(초기화 phase 적대 리뷰 — 하강 4 + false-loud 1). 직전 = **§4.5.258**(generate 안 블록 로컬도 모듈과 같은 규칙). 직전 = **§4.5.257**(초기화는 프로세스가 아니라 **arm 이전 phase** · 상수 fold 제거, **format 26**). 직전 = **§4.5.256**(t0 초기화 순서를 랭크 경로 데이터로 — 축 분리). 직전 = **§4.5.255**(같은-이름 `string` 배열 correct-support). 직전 = **§4.5.254**(t0 정적 초기화 순서 = 모듈 전부 → 블록 로컬 전부·generate 는 모듈보다 먼저). 직전 = **§4.5.253**(§4.5.251 적대 리뷰 — 하강 4건). 직전 = **§4.5.252**(`$sformatf` — 근인은 degenerate `eval` arm). 직전 = **§4.5.251**(`$blk$` decl-init 수집). 직전 = **§4.5.250**(§4.5.248/249 적대 리뷰 — 하강 6건). 직전 = **§4.5.249**(외부 round-20 §6 진단 위치 + §4.11 같은 이름 동적 로컬). 직전 = **§4.5.248**(외부 round-20 8 가족 — fork-arm 블록 로컬·queue 관용구·named arg·`$sformatf`). 직전 = **§4.5.247**(§4.5.246 회귀 수정). 직전 = **§4.5.246**(inner NET shadow — 마지막 ①-급 해소). 그 이전 슬라이스(§4.5.222~245)의 한 줄 요약과 상세는 전부 **[ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)**(인덱스 = 파일 상단, `#### 4.5.<N>` 검색) — 이 문서는 **전방 전용**이므로 완료 서사를 두지 않는다.
>
> **운용 규칙**: 완료 항목은 **즉시 이 문서에서 제거**하고 ARCHIVE로 옮긴다 — 취소선 잔류가 이 파일을 106KB까지 불린 원인이다(잔여가 남은 항목만 "RESOLVED(§x·상세=ARCHIVE) — 잔여 …" 한 줄로 유지).  슬라이스 완료 시 → 상세 로그를 ARCHIVE "완료 슬라이스 로그"에 append(§4.5.x 양식·최신이 위), 이 문서의 해당 잔여 항목 삭제. 신규 발굴은 아래 해당 섹션에 1줄로 추가.


## 요약 (스캔용)

| 순 | § | 주제 | 항목 | 오라클 | 키워드 |
|---:|---|---|---:|:--:|---|
| **1** | §0 | correct-support 승격 큐 | 6 | ✓ 4/6 | **T1 잔여까지 전부 완료(§4.5.222~227)** · **T2-14 `-G` override 도 완료**(§4.5.313 one-shot + §4.5.314 staged — 잔여 = `-pvalue+` 별칭 · `-P<path>=` · 합성-해시 헤더 필드[format bump]) · 남은 것 = T2 잔여(real 정수문맥·리터럴 절단·PART select) + T3-13 `case inside` |
| **2** | §2 | Silent-wrong 잔여 | 41 | ✓ 8 | **폭 인식 상수 접기(3건 동근)** · package real · 구조적 지연 · inner-NET shadow(DEEP) |
| **3** | §6 | G2 OBS 트랙 | 6단계 | 내부 3-way | OBS-2 sva.jsonl → OBS-1 잔여 → R-L4 → OBS-4/5/6 |
| **4** | §3 | Loud→supported 후보 | 35 | ✓ 대부분 | string/heap · 함수/formal · 소형 큐 · VCD fidelity · deep 저우선 |
| **5** | §4 | SVA / 검증 honest-loud | 6 | 일부 無 | empty-match 융합 · N2c · prop-ref skew · N4 clocking · class down-cast |
| **★** | §5 | **✅✅ Phase A~D 전부 완료 — 다음은 [§5.2 재개 지점](#★★★★-52-재개-지점--세션이-끊겼다면-여기부터-2026-08-17)** | — | 실측 | ⭐⭐⭐ **A** 커버리지 **100.00%**(거부 0) · **B** 제품 표면이 **native 하나**(`oracle` feature) · **C** interp = 테스트 도구(성능 최적화 영구 제외) · **D** 성능: 벤치 **8/8 에서 native < vm**(착수 때 셋에서 졌고 최악 **2.52×**). ⭐⭐ **코드젠(cranelift)은 지어서·배선해서·재서 기각** — 런의 **~38% 가 shim** 이고 천장은 op 디스패치 **8.9~11.3%**(§5.1-be). ⚠️ **picorv32 비율은 안 움직였다**(0.61→0.60) = **스케줄러 축은 아직 미측정** |
| — | §7 | 조건부 / 장기 | 4 | — | BACKEND · VHDL · VCD-EXT · MVP-CUT (정확성과 직교) |
| — | §8 | 비계획 | 1 | — | 영구 비목표(DEFPARAM·IMPLICIT-NET·OOS) |

> 🔴 = 열린 silent-wrong(정본 최우선). 취소선/RESOLVED 항목은 **잔여가 있을 때만** 한 줄로 남고 상세는 ARCHIVE에만 둔다.
>
> **순서 주의**: 정본 우선순위(§1)는 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported`인데, 위 표의 1·2위는 **오너 지시로 §0(②)이 §2(①) 앞**에 있다. §0를 먼저 해도 §2의 ①-급이 사라진 것은 아니다.

> **§4.5.275 후속 2줄(2026-07-30 · ②는 2026-07-31 §4.5.277 에서 해소)**: ①output-formal 호출의 eval-order 수리가 **클래스/2-세그먼트 메서드 본문**을 해소하지 못해 `y = c.m(x) + f(x)` 는 정직한 loud — `call_effect` 가 그 본문에 `Inert` 를 증명할 수 있으면 correct-support(오라클 有: iverilog task 쌍둥이 `y=16 x=6`). ②~~frame body 안에서는 copy-out 을 낼 수 없다~~ **RESOLVED(§4.5.277 → 잔여도 §4.5.278·상세=ARCHIVE)** — §4.5.277 이 이름 붙인 조건(**copy-out 의 모든 목적지가 프레임 창 안인가**)은 맞았지만, 그것은 **분류기의 결함**이지 프레임 본문의 성질이 아니었다: `Terminator::Call` 의 copy-out 목적지는 `Stmt` lvalue 가 아니라 call-site 사이드 테이블에 살아서 `compute_suspendable_tasks` 가 한 번도 본 적이 없었다. 워크가 그것을 보게 하니 목적지가 모듈 넷인 형태(bare call · 반환 lvalue · output actual · bit/part-select · 배열 원소 · 문자열 · 중첩 3단 · 루프 안)가 전부 correct-support. **§4.5.277 이 적어둔 "승격 전제 = 그 태스크를 `&mut` 경로로 올리는 것"이 정확히 일어난 일이다.** 잔여 = 프레임 **함수** 본문 안의 output-formal 호출뿐(함수는 `Expr::Call` 로 진입해 자기 소유의 call terminator 가 없다 — 진단이 이제 그 이유를 말한다).

> **§4.5.276 후속 2줄(2026-07-30)**: ①**루프 trip-count 증명이 리터럴만 받는다** — 식별자를 허용하려면 fold 가 net-aware 여야 하는데 `walk_scopes_key_shadowed` 의 계약이 `const_eval_in_scope` 도달 consumer 의 opt-in 을 금지한다(순서 의존 → 과거에 generate body 를 조용히 삭제). 그래서 `for (int j=0; j<NN; j++)`(localparam 경계)·`repeat (NN)` 은 정직한 loud. 승격 전제 = **순서 무관·AST-gathered 이름 집합**(§2 의 inner-NET shadow 와 동일한 선행조건이라 그 슬라이스에 묶는다). ②**`repeat` 카운트가 self-determined 폭으로 절단되지 않는다** — `repeat (8'd128 + 8'd128)` 가 vita 256 / iverilog 0(PRE 동일 = pre-existing silent-wrong). 오라클 有. §2 에 등재.

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

14. ~~elaborate 단계 파라미터 override — `-G<name>=<val>`~~ **RESOLVED**(§4.5.313 one-shot · **§4.5.314 staged**
    — 상세=ARCHIVE). `-G`/`--param` 이 `vita` 와 `velab`(`-L` compose 포함)에 적용되고, `vcmp`/`vrun` 은
    wrong-stage loud 거부이며, `.f` typed bucket 과 `-v` effective echo 의 `params` 행도 있다.
    **잔여 3건:**
    - **`-pvalue+<name>=<val>`** — 미구현(`grep -rn pvalue crates/` = 0건). `-G` 의 별칭 철자이므로 argv 파싱만.
    - **`-P<path>=<val>`**(계층 경로) — 미구현. defparam 이 아직 direct-child 한정이라 같은 제약을 물려받는다.
    - **`.velab` 합성 해시 입력 등록**(doc-14 §RULE B). `-G` 는 지금 **RULE-V upstream 다이제스트에 안 들어간다** —
      섞었더니 `vrun --upstream` 이 바뀌지 않은 `.vu` 를 두고 `E9003 digest changed` 를 내는 **거짓 stale** 이
      됐다(그 필드는 업스트림 입력의 다이제스트지 합성 입력의 것이 아니다). RULE B 를 지키려면 **헤더에
      자기 필드**가 필요하고 그것은 `format_version` bump 다. 지금 상태의 관측 가능한 결과 = 같은 `.vu` 에서
      서로 다른 `-G` 로 만든 두 `.velab` 이 **헤더 128바이트가 동일**하고 어떤 게이트도 구분하지 못한다(본문은
      다르므로 값은 맞다 — 위험은 provenance 뿐이고, `velab` 이 elaborate 를 건너뛰는 경로는 없다).

**T3 — 전제조건 필요 (즉시 착수 대상 아님)**

12. ~~`$fmonitor`/`$fstrobe`~~ **RESOLVED**(§4.5.228·format 25) — 동결 `Monitor`/`Strobe` id 재사용 + `file_directed_stmts` 사이드카. 모니터는 **destination 별**로 유지된다.
13. `case (x) inside {…}` — **no-oracle**(iverilog 13.0이 `case inside`/`inside` op/array reduction 전부 거부) → hand-IEEE + 내부 차분.

**정정 — 재그라운딩에서 stale로 판명(§3에서 삭제, 비목표)**

- ~~package `function string`~~ · ~~package control-flow 함수~~ — 둘 다 **이미 동작**(`hi` / `9` 확인).
- ~~generate 스코프 queue/dyn decl-init~~ — queue **이미 동작**(`4` 확인). 잔여는 string decl-init뿐(위 9번).
- ~~`always @(*)` string concat 조용히 drop~~ — **오진**. vita는 `[abcd]`로 **정확**하고 iverilog가 빈 문자열을 낸다. 명시 `@(a,b)`는 vita가 정직하게 loud. §4.5.217 리포트의 이 줄은 잘못 기록된 것.

> **주의(정직한 순위 고지)**: 프로젝트 정본 우선순위(§1)는 **① 오라클 있는 CRITICAL silent-wrong > ② loud→supported**다. (아래 §0-B가 ①-급으로 걸어 두었던 **inner NET vs outer PARAM shadow**는 **§4.5.246에서 RESOLVED** — 2026-07-30 확인.)

## 0-B. NEXT — 재개할 deep-defer follow-on

> **round-18 리포트 8-가족 RESOLVED(§4.5.213·2026-07-24)** — 외부 리뷰어 round-18 리포트의 잔여 8-가족(A/G queue/array-of-non-packable-record + foreach·D automatic-block-local-init·E1 enum-method-on-formal·E2 struct-member-method + string-dyn-element·F1 output-formal-fn-in-loop-cond·F2 severity-in-frame-body·F3 wrapped-dyn-formal) + C1 const-repeat를 correct-support화(hand-IEEE/iverilog 차분). 상세=ARCHIVE §4.5.213.
>
> **C1 part 2: fork-in-frame RESOLVED(§4.5.214·2026-07-24)** — `fork…join[_any|_none]`을 suspendable(framed) task 내부에서 실행하는 것이 "깊은 스케줄러 rework·blast radius=frame 서브시스템 전체"라 correct-or-loud LOUD 유지 중이었으나, 재조사 결과 **단일-스레드 스케줄러 + 기존 `stash_frame_windows`/`restore_frame_windows`가 이미 concurrent children을 parked parent로부터 격리**하고 있어 arm이 부모 frame-local을 안 건드리는 **Case A**(리포트 repro)는 기존 owned-window 모델로 즉시 동작함을 확인 — 신규 인프라가 필요한 건 arm이 부모 frame-local을 read/write하는 **Case B**뿐(interior-mutable arena `WindowSlot::Shared`, dyn_heap/class_heap과 동형). 3-stage(Case A·Case B `join`-all arena·Case B `join_any`/`join_none` refcount)로 전달 + final-review가 fork arm 내부 `return`의 silent frame-corruption 회귀를 잡아 즉시 loud화. format 23 불변. 상세=ARCHIVE §4.5.214.
>
> **§0 NEXT 최상단** — ~~inner NET이 outer PARAM/enum-label을 shadow 못 함~~ **RESOLVED(§4.5.246, 회귀 수정 §4.5.247)**. 형제 항목(package 변수 clobber·block-local 잔여)은 §2에 남아 있다.
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

> ⚠️ **2026-08-03 오너 지시 — 위 4단계 위에 성능 T 단계가 올라간다.** 근거는 §4.5.282 의 실측:
> ③층 격차가 **76×**(verilator, 같은 설계·기계)이고, 동시에 **②층에 10.7× 가 미청구**로 남아
> 있다 — `is_codegen_able` 이 `Terminator::Call` 을 가진 프로세스를 통째로 거부하고
> `codegen_coverage` 는 `ir.processes` 만 보므로, **사용자 함수를 부르는 RTL 에서 바이트코드 VM 의
> 기여가 정확히 0%** 다(`--backend interp` 와 `bytecode` 가 같은 시간). 정확성 큐는 사라지지 않고
> **T 단계 뒤로 밀린다**.

현재 NEXT 큐(상세=LOOPROMPT · 스캔용 표 = 문서 상단):

0. ★★ **③층 S1d-4a — `impl Kernel`** (§5 · 정본 = [preview/21 §5 S1 분해 표 + S1d-4 그라운딩 블록](preview/21-tier3-native-backend.md)).
   **그라운딩이 계획을 바꿨다(2026-08-03)**: ③층 실행기는 두 번째 실행기가 아니라 **`Kernel` 의 두
   번째 구현자**다 — `compute_effect`/`apply_effect` 가 이미 `K: Kernel` 제네릭이라 문장 단위 의미가
   **전부 재사용**되고(`$display`·NBA·형변환) byte-identity 가 미러가 아니라 **구조적**이 된다.
   `run_process`(바디 워크)만 `&mut Scheduler` 고정이라 4b 로 분리. 52 메서드 = 코어 ~16 + 게이트
   거부 ~9 + **~27 = "퍼널 밖 효과" 선결 과제**(트레이트가 구조적으로 강제 — 노트로 들고 있을 수 없다).
   T0·S0(§4.5.285)·S1a~c(§4.5.286/287)·S1d-1(§4.5.288)·S1d-2(§4.5.289)·**S1d-3(§4.5.290)** 완료 —
   저장·read-path·쓰기 퍼널·dirty/edge 채널·**wake 결정**이 전부 엔진과 차분 일치한다.
   **S1d-4 범위** = Active/Inactive/NBA 리전 큐 + 델타 루프 + **in-body 웨이터**(`WaitCause::Edge`/
   `Expr`·`Level{arm=Some}`)와 **`busy` 유지자** + cont-assign settle + wired-AND/OR 다중 드라이버
   해소. 게이트 = corpus 적격분 **stdout+VCD 바이트 동일**(원래 S1 게이트).
   ⚠️ **`busy` 는 적격 설계에서 실제로 참이 된다**(§4.5.290 실측: `always @(posedge clk) begin …
   @(negedge rst); end` 이 S0 적격·아레나 빌드 가능인데 엔진 busy 가드가 wake 를 막는다) — 같은
   설계가 in-body 웨이터 모델도 요구하므로 둘은 한 슬라이스다.
   ⚠️ **VCD 는 gate-reject 가 아니다** — emitter 는 **store 지점**에 있어야 하고 `note_change` 가
   `word` 를 되찾아야 한다(sweep 시점이면 슬롯 내 A→B→A 가 한 레코드로 합쳐진다 · `native/dirty.rs`).
   ⚠️ **남은 필수 1건**: 퍼널 밖 효과 배선(`rhs_is_stmt_effect` 가족 + `$readmem*`/`$sformat` ·
   `r = $random(seed)` 는 **오늘 적격**). **T4** 는 기회 슬라이스로 유지.
1. **§0 T2 잔여 2건** — `real` const-fold · sized-literal enum label(각자 독립 슬라이스). generate/iface string decl-init·음수 range bound·`$fmonitor`/`$fstrobe`·T1 전부 완료(§4.5.222~228). `real` const-fold 는 §4.5.229 가 남긴 `int'(<real param>)` 바운드의 선행이기도 하다.
2. **§2 오라클-有 silent-wrong** — ~~part-select 바운드 + replication count~~ **RESOLVED**(§4.5.229). 남은 것 = **폭 인식 상수 접기**(위 "상수 폭 잔차" ①②③ 3건이 전부 동근 — 인터프리터 coerce 가 가장 도달성 높음) · package-scope real · **구조적 지연**(§4.5.221이 도달성을 넓혀 우선순위 상향 후보) · real→`input int` formal.
3. **§2 DEEP** — inner NET vs outer PARAM shadow(**선행 = order-INDEPENDENT AST-gathered per-scope name set**; 없이 켜면 §4.5.218 S1 재발) + 형제 항목(package 변수 clobber·block-local 잔여 2형).
4. **OBS-2 sva.jsonl**(R-L6).

## 2. Silent-wrong 잔여 (1건 제외 전부 pre-existing·baseline 동일 — deep defer 또는 기록됨)

> **오라클 있는 것부터 위로.** 아래 🔴 중 A1~A7(오라클 ✓)이 §1 우선순위 ①에 해당하고, 무오라클/soundness 발굴분은 그 아래.

- 🟡 **`%h` 가 1비트 미지 EXPRESSION 결과를 `x` 로, iverilog 는 `X` 로 찍는다** (§4.5.326 차분에서 발견·PRE==POST·이 슬라이스와 무관) — `$display("%h", ^a)` 에서 `a` 에 x 가 있으면 vita `x` / iverilog `X`. **같은 값의 1비트 NET 은 양쪽 다 `x`**(`reg [0:0] a = 1'bx`) 이므로 iverilog 자신이 일관되지 않고, IEEE §21.2.1.3(*"모든 비트가 같은 미지값이면 그 미지값(소문자), 섞이면 대문자"*)은 vita 편이다. 값이 아니라 **렌더링**이며 `%b`·`%0d` 는 일치한다. 승격 조건 = 제2 오라클이나 LRM 재독으로 판정을 굳힐 때. 215설계 중 17칸.

- ~~**🔴 사이즈 캐스트 `N'(expr)` 의 문맥 규칙**~~ **RESOLVED**(§4.5.316·상세=ARCHIVE) — IEEE §11.8.1 의 `max(self, N)` + 부호 무조건 전파. **잔여 6건**(전부 적대 리뷰 실측):
  - ⓐ **너비 프로브가 진단을 두 번 낸다**(§4.5.316 이 만든 품질 회귀·값은 정확): 좁힘 판정을 위해 노드를 한 번 낮춰 `ir_bits_of` 만 읽는데, 그 lowering 이 **진단과 사이드테이블 등록을 전부 실행**한다 — `8'(a >> pk::nope)` 가 E3009 **×2**, `2'({u1.nope} % 4)` 가 E3010 ×2(지연-hier 리졸버에서), `MAX_ELAB_ERRORS=200` 캡도 두 배로 먹는다. 그리고 죽은 노드가 **아티팩트에 직렬화**돼 깊이 32 중첩에서 `.velab` 가 **5.7×**(5884 B vs 1035 B) 커진다. §4.5.317 이 real 거부를 얹으면서 **`%`·`>>`·`>>>` 는 캐스트당 2건**이 됐다(프로브의 기존 일반 real 진단 + 새 진단 · iverilog 는 1건 · `<< <<< ** / * + -` 와 leaf 는 1건 = 최대 +1). **정본 해소 = 낮추지 않고 `w` 를 답하는 AST self-폭 패스**(그러면 프로브 자체가 사라지고 이 중복도 같이 사라진다).
  - ⓑ `w = ir_bits_of(plain)` 은 **노드**의 폭이라 캐스트 피연산자 **전체**의 self 폭을 못 본다(더 넓은 형제가 올린다 — `2'((s8>>u3)*s16)` vita `11` / 두 오라클 `01`, 11칸).
  - ⓒ **폭을 모를 때는 어느 기본값도 옳지 않다 — 시도했고 되돌렸다**(2026-08-08). **증상은 concat 래퍼가 없어도 난다**(적대 리뷰 실측·오라클 ✓): `2'(u1.mem[0] % 4)`·`2'(u1.k[7:0] % 4)` 가 vita `xx` / iverilog `11`, `string s="A"` 에서 `2'(s % 4)` 가 `xx` / hand-IEEE `01`. 즉 *"게이트가 bare 계층 피연산자를 거절한다"* 는 **whole-net 읽기에만** 참이고, 원소·비트/부분선택·string 은 그대로 arm 에 들어온다. `ir_bits_of` 가 `None` 을 내는 곳(실측): 지연-hier 읽기(whole-net·원소·비트선택) · `string` 넷 · **string 을 내는 `SysFunc` 가족**(`$sformatf`·`s.substr()`·`s.toupper()`). ⚠️ **클래스 필드·dyn 배열 원소·로컬 호출은 `None` 이 아니다**(초판 기록이 틀렸다 — 두 검출기로 실측). 계층 **호출**은 양쪽 다 loud 라 도달 불가.
  이제 `unwrap_or(n)` 은 **넓힘 가지**를 골라 이 arm 이 고치려던 좁힘 결함을 재현한다. **그러나 `unwrap_or(u32::MAX)` 로 좁힘 가지를 기본값 삼으면 더 나빠진다**: ⓐ `w` 는 가지 선택뿐 아니라 좁힘 가지의 **fill 문맥**으로도 쓰여(`lower_ctx_or_plain(lhs, w)`) `4'('0 / {u1.k})` 가 `0000` → **`xxxx`**(`fill_literal_const` 이 `width=u32::MAX` 상수를 만들고 `alloc_width` 는 캡을 건다 = 비일관 상수 → W4025 → X) · `.velab` **22.4×** · RSS **10 MB → 1.1 GB** ⓑ 좁힘 가지는 노드를 **자기 폭**으로 남기는데 `lower_size_cast` 는 **자기 몫의 `unwrap_or(32)`** 로 판단하므로 `32'({u1.k[7:0]} / d4)` 가 **8비트**를 낸다(넓힘 가지는 항상 `n` 비트라 우연히 정합이었다 · replicate 없이도 재현). 실측 회귀 41칸.
  ⚠️ **`unwrap_or(32)` 는 fill 축에서 4/4 정답**(`u32::MAX` 는 0/4)이지만 **가지 축에서는 다르다**(참 폭이 32 초과면 다시 좁힌다) — 한 변수가 두 질문에 답할 수 없다는 것이 이 항목의 결론이고, 저장소에는 이미 같은 뜻의 `unwrap_or(32)` 관례가 네 자리에 있다.
  **⇒ fallback 선택 문제가 아니다** — `w` 를 **알 수 있게** 만드는 것(= 낮추지 않고 답하는 AST self-폭 패스, ⓐ와 같은 선행조건)만이 답이고, 그것이 서면 프로브·진단 중복·아티팩트 팽창이 함께 사라진다.
  ⚠️ 초판 기록의 *"피연산자 확장은 값을 보존하므로 몫이 안 바뀐다"* 는 **일반적으로 거짓**이다(`b=-4'sd8, c=-4'sd1` 에서 `8'(b/c)`=8 · `4'(b/c)`=−8, 두 오라클도 그렇다) — 도달 가능 집합이 전부 `ext == false` 라서 **공허하게** 참이었을 뿐이다. 같은 이유로 초판이 `>>` 를 "잔여" 라 적은 것도 틀렸다: `2'({u1.k} >> '1)` 는 main `01` / 되돌린 구현 `11` = iverilog 라 그 절반은 **수정**이었다.
  - ~~ⓓ **넓힘 가지의 fill 이 깨진다**~~ **RESOLVED**(§4.5.318 — `lower_size_leaf` 가 fill 을 `n` 에서 짓는다).
  - ~~ⓔ **`**` 의 지수가 밑수의 부호를 따른다**~~ **RESOLVED**(§4.5.319·상세=ARCHIVE) — 근인은 이 기록이 지목한 `lower_expr_ctx` 가 아니라 **엔진**이었다(`eval_core.rs` Pow 팔의 `exp.signed = base.signed`). 지목된 자리도 별개의 live defect 여서 함께 고쳤고(`logic [7:0] r = a ** '1` → vita 0 / 오라클 2), `expr_size_ctx` 의 `Pow` 프로브-가지 이동도 그때 회귀 0·FIXED 72 로 착지했다. **잔여 = 아래 "지수 자기결정 CLASS" 항목.**
  - ⓕ **4-state 좁힘이 x 를 떨어뜨린다**(`a=8'bxxxx_0011` 에서 `2'(a+1)`·`2'(a*2)`·`2'(-a)`·`2'(a-1)` 이 전부 known / iverilog `xx` · `<<`·`&` 는 정상) — 저비트 폐쇄가 **2-state 논증**이라는 것의 대가.
  - ⚠️ **생존 뮤테이션 5**(리뷰 실측): `n >= w` → `n > w` 는 **죽지 않고 오히려 fill 칸 3개를 더 맞힌다**(`>=` 쪽에 근거가 없다) · `unwrap_or(32)`(ⓒ) · 넓힘/좁힘 가지의 **시프트 양을 문맥결정으로** 바꾸는 둘(킬러 설계는 리뷰가 제시했다) · `narrow_op` 와 `coerce_sign` 은 **서로 중복**이라 신규 테스트가 둘의 **논리합만** 핀한다.
- **🔴 사이즈 캐스트 주변의 pre-existing 셋(PRE==POST·같은 슬라이스가 실측)** — ⓐ **저비트 폐쇄는 2-state 논증이다**: `logic [7:0] a=8'bxxxx_0011` 에서 `2'(a+1)` 이 vita `00` / iverilog `xx`(캐스트 없는 `a+1` 은 vita 도 `xx` 로 정확 = 좁히기만의 문제). 비트연산과 `<<` 는 4-state 에서도 폐쇄(4,116칸 발산 0)라 절반만 틀린 논증이다. ~~ⓑ **`**` 는 음수 지수에서 폐쇄가 아니다**~~ **RESOLVED**(§4.5.319) — `Pow` 가 `Div|Mod|Shr|AShr` 폭-프로브 가지로 옮겨졌다(회귀 0 · FIXED 72). ⓒ **fill 리터럴이 `lower_size_ctx` 를 거치면 캐스트 문맥을 잃는다**(`8'(a+'1)` vita `…110` / 오라클 `…100` · `8'('1>>1)` vita `0` / 오라클 `01111111` · **비트연산 철자도 같다**: `4'('1 ^ a)` a=4'hD 에서 vita `1100` / iverilog `0010`, `9'(('1+one) % (sb**3))` vita `000000010` / iverilog `000000000` — 캐스트 없이는 둘 다 일치) — Size arm 주석이 *"fill 은 N 으로 자란다"* 라고 적은 바로 그 경로를 `is_size_ctx_operation` 게이트가 우회시킨다. ⚠️ **HEAD 에서 ⓒ가 재현되지 않는다**(§4.5.339 착수 재현: 72칸 매트릭스[9 연산자 × fill 2 × 위치 2 × 대상폭 2] **발산 0** · 인용된 네 철자 `8'(a+'1)`·`8'('1>>1)`·`4'('1 ^ a)`·`9'(('1+one) % (sb**3))` 전부 iverilog 일치) — 중간 슬라이스가 부수로 닫았다. ⓐ·ⓑ 는 유효.
- **🔴 크기 캐스트가 파라미터·패키지 파라미터·함수 호출 leaf 를 못 본다 — 그리고 고치려면 AST self-폭 패스가 먼저다**(pre-existing·PRE==POST·오라클 ✓·§4.5.318 이 지어서 재고 **되돌렸다**). `ast_ctx_signed` 가 leaf 하나라도 `None` 이면 캐스트가 문맥 하강을 **통째로** 건너뛰므로, 그 leaf 종류에서는 §4.5.212 의 carry 결함이 그대로 살아 있다 — 63칸 매트릭스에서 **35칸 발산**(`8'(P*Q)` = 13 / 오라클 45 · `16'(P<<4)` = 0 / 오라클 3840 · 넷과 `localparam int` 는 0/9). ⚠️ **해석기를 붙이는 것만으로는 안 된다**(전부 실측):
  - ⓐ 붙이면 그 트리가 `lower_size_ctx` 로 들어가는데, **안쪽 부분식의 self 폭이 `n` 을 넘으면 폭 모델이 틀린다**(§4.5.316 의 프로브도 `n` 에서 다시 잰다) → 6,720칸 중 **42칸 correct→silent-wrong**(`4'(13 + (PS>>2))` PRE −4 = 두 오라클 / POST 0 · net 쌍둥이는 PRE 도 틀림 = 맞바꾸기). **선행조건 = 낮추지 않고 트리 전역 self 폭을 답하는 AST 패스**(§2 의 ⓐ·ⓑ·real+fill 84칸과 같은 뿌리).
  - ⓑ **함수 호출 leaf 는 `extend_to` 가 두 번 참조한다**(`Select{base:e}` + `Concat[fill,e]`) → `40'(rnd(0)+0)` 이 함수를 **두 번** 부르고 두 `$random` draw 를 한 값에 **섞는다**(exit 0). §4.5.310 이 인덱스에서 겪은 그 부류 — **복제 가능성 술어로 게이트**하거나 값을 한 번 묶어야 한다.
  - ⓒ **함수 formal·함수/블록 local 은 결합 바인딩 집합에 없다** → lowering 을 흉내 내 결합 키를 먼저 잡으면 그것들을 지나쳐 **동명 모듈 파라미터의 부호**를 가져온다(52칸 중 11 회귀). 반대로 넷 우선(현재)이면 generate 스코프 localparam 이 바깥 넷에 가린다(`16'(X*S)` = 3 / iverilog 195, PRE==POST). **어느 고정 순서도 규칙이 아니다** — lowering 의 실제 결정 절차를 그대로 따라야 하고, 거기엔 `subst_lookup`/`out_subst_lookup` 같은 인라인 치환 경로가 더 있다.
- **🔴 크기 캐스트의 real 이 fill 과 만나면 아직 조용하다**(pre-existing·PRE==POST·오라클 ✓·§4.5.317 적대 2렌즈 실측). §4.5.317 이 **퍼널이 만드는 모든 피연산자**를 막았지만, 판별자 **셋이 동시에 성립**하면 퍼널에 애초에 안 들어간다 — 반대편이 **fill(`'0`/`'1`)** ∧ real 원천이 **평범한 real 넷이 아님**(`parameter real`·real 리터럴·real 함수 반환·`$signed(r)`·`$realtime`·`$sqrt(r)`) ∧ 연산자가 **real 비전파**(`& | ^ << >> >>> %`). 288칸 매트릭스 중 **84칸**(`4'(RP ^ '0)`·`4'($sqrt(r) & '1)`·`4'(b >>> (rt ^ '0))` 이 전부 exit 0·iverilog 는 전부 거부). 두 자물쇠가 같이 열려야 샌다: `ast_ctx_signed` 가 그 leaf 에서 `None` 이라 `lower_size_ctx` 를 **안 타고**, `expr_is_real` 의 `Binary` 팔에는 **비트/시프트/`%` 가 없다**. 하나만 고치면 다른 하나가 남으므로 **인스턴스가 아니라 CLASS** 이고, 선행조건은 위 ⓐ/ⓑ 와 같은 **트리 전역 AST self-폭/도메인 패스**다.
- **`$signed(real)`/`$unsigned(real)` 이 위치 의존적이다**(pre-existing·PRE==POST·§4.5.317 발굴). 캐스트 안 15자리는 거부하지만 7자리는 exit 0 — 평범한 산술(`$signed(r)*2` → 15)·`%0d`/`%0f` 포맷·int/real 대입. iverilog 는 `The argument to $signed must be a vector type` 로 **전부** 거부한다. 곁: **2인자 `$signed(r, u)` 를 조용히 받는다**(iverilog: *"takes exactly one(1) argument"*) — §4.5.317 은 그 형태에서 **어느 슬롯이든** real 이면 캐스트를 거부하도록 맞췄지만, 스펠링 자체의 거부는 별건이다.
- ~~**🔴 elaborate 상수접기가 `**` 지수를 자기결정하지 않는다**~~ **RESOLVED**(§4.5.339·상세=ARCHIVE) — 세 fold 사이트가 **한 헬퍼**(`const_pow_exponent_selfdet` → §4.5.186 폭 모델의 `eval_const_env_self`)를 공유한다. §4.5.319 가 닫은 다섯 철자에 이은 **여섯 번째이자 마지막**. 97칸 3-way **FIXED 10 · LOUD→CORR 2 · WRONG→LOUD 2 · 회귀 0**. **잔여 = 아래 「폭-미상 wrapping 지수」 한 줄뿐**(genvar 부호는 그 슬라이스가 함께 고쳤다).
- **🔴 real 도메인의 `**` 가 정수 지수를 넓힌다**(pre-existing·오라클 ✓·§4.5.339 발굴). `parameter real R = 2.0 ** -4'sd8` 이 vita **256.0** / iverilog **0.003906** — `const_real.rs` 의 `a.powf(b)` 가 지수를 `const_eval_real_in_scope` 로 읽어 `-4'sd8`(4비트 `1000`)이 +8 로 승격된다. **정수 지수는 §4.5.339 의 공유 헬퍼가 이미 답할 수 있다**(퍼널이 있으니 arm 하나) · real 지수(`2.0 ** 0.5`)는 §11.8.1 그대로 실수.
- **폭-미상 leaf 위의 WRAPPING 지수는 무제한 fold 로 남는다**(§4.5.339 가 **일부러** 남겼다·리뷰 실측). 폭이 0(=UNKNOWN)으로 기록되는 leaf(다중 packed 로컬 등) 위에서 지수가 **랩하면** 값이 갈린다 — `3 ** (m - 8'd250)`(m=4)이 vita **0** / iverilog **59049**. ⚠️⚠️ **거부는 답이 아니다 — 지어서 재고 되돌렸다**: 폭-미상을 거부하면 range-bound 위치에서 **decline 경로가 조용한 기본값**을 넣어(`logic [f3():0]` → **1비트 넷·exit 0·진단 0**) correct→**silent-wrong** 이 되고, 값이 정확한 셀(`3 ** (m + 0)`)은 correct→loud 로 강등된다 ⇒ **해소는 거부가 아니라 폭 모델 완성**(`const_decl_wsign` 이 다중 packed 를 답하게).
- **🔴 module-scope `$clog2` 가 인자를 무제한으로 접는다 — 인터프리터는 안 그런다**(pre-existing·오라클 ✓·§4.5.339 발굴). `localparam L = $clog2(4'd15 + 4'd1)` 이 vita **4** / iverilog **0**(인자는 자기결정 4비트라 15+1=0). `eval_const_env_at` 의 SysCall arm 은 이미 자기결정이라 **같은 소스가 상수함수 안에서는 0** = 두 답. `const_eval_in_scope` 의 `$clog2` arm 을 §4.5.339 의 Pow 와 **같은 기계**로 접으면 된다.
- **🔴 replication count·part-select 폭 lane 이 `**` 를 아예 안 접는다**(pre-existing·오라클 ✓·silent·§4.5.339 발굴). `{(8'd2 ** 8'd3){1'b1}}` 이 vita `00000000` / iverilog `11111111` · `v[0 +: (8'd2 ** 8'd3)]` 가 `01` / `cd`. `const_bound_u32` 의 게이트 조건 3(width-growing op 은 leaf 가 ≥32비트여야)에서 거절되고 소비자가 `unwrap_or(1)`/`unwrap_or(0)` 로 **조용히** 떨어진다.
- **🔴 캐스트의 SIZE 식이 무제한으로 접힌다**(pre-existing·오라클 ✓·§4.5.339 발굴). `4'd3 ** ((4'd9+4'd8)'(2))` 가 vita **9** / iverilog **1**(size 식이 자기결정 4비트라 **1** 로 랩 → `1'(2)`=0 → 3^0). 자리 = `cast_size_bits` + `const_self_width` 의 `Cast::Size` arm — **§4.5.339 가 그 arm 에 Named 철자를 더했으므로 두 철자가 같은 결함을 공유한다**(함께 고칠 것).
- **u64 패턴 지수를 i64 도메인이 음수로 읽는다**(pre-existing·오라클 ✓·§4.5.339 발굴). `4'sd3 ** (64'd0 - 64'd8)` 이 param **0** / iverilog·vita 런타임 **926288481** — `const_eval_i64_lit` 의 64비트 재해석 arm 이 −8 을 만든다(그 arm 주석은 *"magnitude misuse 는 range 검사가 loud 로 잡는다"* 인데 **지수 위치엔 그 검사가 없다**).
- **인터프리터의 폭-0(=UNKNOWN) 타깃이 실폭처럼 마스킹에 참여한다**(pre-existing·오라클 ✓·§4.5.339 발굴). `bit [1:0][3:0] tt; tt = 4'd13 ** 4'd2;` 가 **9** / iverilog **169** — `eval_const_assign` 의 `w.max(tw)` 가 `tw=0` 을 그대로 쓴다(**자기 doc 이 반대로 적어 뒀다** — *"width 0 = UNKNOWN ⇒ 마스킹 안 함"*).
- **fold 안 되는 body-local 초기화가 조용히 0 이 된다**(pre-existing·오라클 ✓·§4.5.339 발굴). 상수함수 본문의 `int x = 8'(5); g = x;` 가 vita **0** / iverilog **5** — `.and_then(...).unwrap_or(0)` 이 *"초기화 없음"* 과 *"초기화 미fold"* 를 합친다(`eval_const_call` 자기 doc 의 *"None → LOUD"* 와 모순 · 사이트 둘). 곁: `int t = f();` 형태의 body-decl 초기화는 **PRE·POST 둘 다 스택 오버플로**(깊이 캡이 그 경로를 안 묶는다 = §4.5.339 가 인자·default 철자만 닫았다는 증거).
- **self-referential 반환 range 는 여전히 스택 오버플로**(pre-existing·오라클 없음 — iverilog 도 내부 abort·§4.5.339 발굴). `function [f():0] f();` — `const_fn_ret_wsign` 경로가 call 깊이를 안 끈다. 처방은 §4.5.339 가 default 에 쓴 **같은 한 줄**(`depth + 1`).
- **`coverpoint_domain` 의 Pow arm 이 정본을 미러한다는 주장과 어긋난다**(pre-existing·minor·§4.5.339 발굴). 그 함수는 `max(lw,rw)`/`ls && rs` 로 접는데 정본(`ir_bits_of`·`sim-ir::selfwidth`)은 **LHS 폭·base 부호**다. 영향 = 커버리지 auto-bin 개수뿐.
- **기본 백엔드의 Mul 체인이 밑수를 n 번 재-lower 해 진단을 n 배로 낸다**(pre-existing·PRE==POST·§4.5.319 라운드 3 발굴). `native_eval` 이 `a ** n`(작은 상수 n)을 Mul 체인으로 펼칠 때 `lhs` 를 n 번 낮추므로, 밑수에 범위 밖 배열 읽기가 있으면 `always @(posedge clk) r <= m[idx] ** 16;` 이 interp/native 에서 **E4002 2건**, bytecode 에서 **8건 + "further suppressed"**(8-cap 소진). **값은 안 틀린다**(체인 leaf 집합이 순수 — `Expr::Call` 은 명시 거부) — 갈리는 것은 진단 수뿐이고, 그것이 8-cap 을 먹어 뒤쪽 진단을 지운다. 같은 클래스가 `native/dirty.rs` 에 이미 기록돼 있다.
- ~~**🔴 좁히는 사이즈 캐스트의 결과가 더 넓은 대입 문맥에서 절단되지 않는다**~~ **RESOLVED**(§4.5.320·상세=ARCHIVE) — 캐스트가 `$signed`/`$unsigned` 로 **자기결정 경계를 실체화**한다. 그 슬라이스가 남긴 잔여는 아래 다섯 줄(전부 "elaborate 가 피연산자의 모양을 틀리게 안다" 한 CLASS).
- **🔴 prim cast 가 타깃 폭을 문맥결정 피연산자에 내리지 않는다**(pre-existing·PRE==POST·오라클 ✓·§4.5.320 발굴). `a=8'hFF` 에서 `int'(a*a)` 가 vita `00000001` / iverilog `0000fe01`, `shortint'(a*a)` 가 `00000001` / `fffffe01`. size cast 는 §4.5.212 이후 `lower_size_ctx_entry` 로 내리는데 `lower_prim_cast` 는 `lower_ctx_or_plain`(fill 만)이다. **그대로 배선하면 회귀한다** — prim cast 는 real 피연산자를 합법으로 받는데(`int'(r)` = 반올림) `refuse_real_size_operand` 가 그것을 loud 로 만든다. 선행조건 = 아래 real 항목과 같은 **AST 도메인 패스**.
- **🔴 `ir_bits_of` 가 클래스 필드의 폭을 핸들넷에서 읽는다**(pre-existing·§4.5.320 발굴). `try_class_field_read` 가 `Signal{net: 32비트 핸들넷, word: field-id}` 를 만들고 진짜 폭은 `class_field_widths` 사이드카에만 둔다 — `ir_bits_of` 는 사이드카를 안 보므로 **틀린 `Some(32)`** 를 낸다(`16'(c.sb)` = `xxxd`, hand-IEEE `fffd`). §4.5.320 은 캐스트에서 **정본 폭과 대조해 봉인을 거절**하는 것으로 막았을 뿐 `ir_bits_of` 자체는 그대로다 — 소비자가 많아 **CLASS**, 정본은 `canonical_self_width`. 같은 뿌리의 두 번째 증상(라운드 4 실측·PRE 동일): 캐스트 폭이 지어진 32 와 **같아지면** `Ordering::Equal` 이 resize 를 통째로 건너뛰어 캐스트가 사라진다 — `32'(c.s8)` = `fd`(8비트·hand-IEEE `fffffffd`) · `32'(c.s8 + ua[0])` = `fa`(캐리 소실·`000001fa`).
- ~~**🔴 `resize_inline_assign` 은 봉인 안 된 쌍둥이다**~~ **RESOLVED**(§4.5.321·상세=ARCHIVE) — 스탬프가 무조건이 되어 선언 반환 폭이 봉인한다. **잔여 넷**(전부 pre-existing·PRE==POST·적대 2렌즈 실측):
  - **🔴 인라인 경로가 선언 폭을 본문 안으로 안 내린다 — 그리고 프레임 경로는 내린다**(오라클 ✓). `function [31:0] fh(input [7:0] x); fh = fld * x;`(fld = 8'hFF) 가 static 에서 `00000001`, **`automatic` 에서 `0000fe01` = iverilog**. 같은 본문이 lifetime 철자 하나로 갈린다. 근인 = `lower_ctx_or_plain(rhs, ctx_w)` 는 **fill 만** 크기를 주고 문맥결정 연산에는 §4.5.212 를 적용하지 않는다 — prim cast 항목과 **같은 클래스**(폭이 피연산자에 안 내려간다).
  - ~~**🔴 인라인 인자 바인드가 formal 의 선언 타입을 적용하지 않는다**~~ **RESOLVED**(§4.5.325·상세=ARCHIVE) — 폭·부호·2-state 를 한 게이트로 함께 적용한다(1,920칸 FIXED 636·REG 0). **잔여 = 아래 다섯 줄**(전부 같은 바인드가 *못 고친* 자리이고 각각 다른 퍼널이다).
  - **🔴 프레임 인자 바인드가 §11.6.1 확장 부호를 안 쓴다**(오라클 ✓·§4.5.325 실측). 좁은 signed actual 이 더 넓은 **unsigned** formal 로 갈 때 0확장한다 — `function automatic [31:0] ff(input [15:0] x); ff = x;` 에 `8'shf7` 이 `000000f7` / iverilog `0000fff7`. **세 퍼널이 공유**한다(프레임 함수·프레임 태스크·클래스 메서드)라 CLASS 이고 별도 슬라이스. ⭐ 판별: 평범한 넷 대입 `u = 8'shf7` 과 **모듈 포트 연결**은 이미 정답이므로 문맥 하강이 아니라 **바인드**가 자리다. 1,920칸 중 24칸.
  - **🔴 `expr_is_repeatable` 이 배열 원소를 거절해 `f(mem[i])` 가 바인드를 못 받는다**(오라클 ✓). 상수 인덱스여도 `Signal{word:Some}` 이면 거절(중복 언급이 E4002 를 두 번 낸다 — §4.5.312)이라 signed/2-state formal 에 배열 원소를 넘기면 pre-slice 바인드가 남는다: `gs(arr[2])` 가 `00…f7` / iverilog `ff…f7`. 필요한 것은 반복 가능성이 아니라 **부작용 없는 중복**(진단 1회 보장). `f(mem[addr])` 는 일상 관용구라 실blast 는 "불순 actual" 보다 훨씬 넓다.
  - **🔴 계층 참조·클래스필드 actual 은 선언 폭 자체를 못 받는다**(PRE==POST). placeholder/지어진 32 라 `trusted_self_width` 가 `None` → 바인드 전체가 사퇴하고, 호출 결과가 선언 폭이 아니라 **actual 폭**으로 나온다(`function [63:0] gs(input [7:0] x); gs={x,x};` 에 `hi.hv` → 8비트 · 본문이 `x*x` 면 8비트 산술). generate 스코프 이름도 같다. 위 "지어진 폭" 항목과 **같은 퍼널의 다른 철자**.
  - **🔴 `cast_operand_is_real` 의 AST 절반이 bare 단일 세그먼트만 본다**(오라클 ✓·§4.5.325). `pa(f(0))` 는 4(정답)인데 **같은 callee** 를 `pa(p::f(0))` 로 쓰면 f64 페이로드가 2-state formal 로 들어간다(`c.cm()` 도). 한 설계가 한 callee 를 두 가지로 답한다 = §4.5.310 의 "철자로 인식" 재발. 넓히면 호출부 8곳을 건드리므로 별도 슬라이스.
  - **⚠️ 4-state actual → 2-state formal 의 강제가 런타임 O(선언폭)**(§4.5.325 실측 · 3백엔드 동일 · 폭에 선형: `byte` 12.8× `shortint` 23.8× `int` 46.4×). `coerce_two_state` 가 비트마다 피연산자를 언급하고 엔진이 DAG 를 트리로 걷는다. **알려진 값이면 안 짓는** 술어가 절반(2-state actual)을 지웠고, 나머지 절반은 `function int f(input logic [31:0] x)` 라는 흔한 철자다. 진짜 수정 = x/z→0 **IR 프리미티브 하나**(format bump) 또는 엔진 memoize. 같은 비용이 `expr_cast.rs` 캐스트 경로에 pre-existing. ⚠️ 완화 둘(per-query 메모·노드 예산)을 지어서 **둘 다 개선 0** 으로 반증했다 — 비용은 깊은 워크 하나가 아니라 **바인드 개수**에 있고(인라인 fold 가 호출마다 별도 서브트리를 만들어 아레나가 진짜로 2^n 개 노드), 영속 캐시는 in-place 패치(placeholder 는 패치 전에 "알려진 1비트 Const" 로 읽힌다)와 충돌한다.
  - **🔴 인라인 body-local 의 2-state 선언은 x/z 를 안 떨어뜨린다**(오라클 ✓·PRE==POST). 함수 본문 안 `bit [7:0] b; b = x;` 가 `x7` / iverilog `07`. `fold_straight_line` 이 폭·부호는 적용하고 2-state 단계만 없다 — 바인드와 **같은 IEEE 규칙의 두 번째 철자**라 그 둘은 함께 한 자리로 합쳐야 한다.
  - ~~**🔴 상수 인덱스 carve-out 이 인덱스를 무부호로 읽는다**~~ **RESOLVED**(§4.5.324·상세=ARCHIVE) — signed 상수를 인덱스 도메인으로 부호확장한다. **위 인라인 바인드 항목의 선행조건이 이것으로 섰다.** 잔여 두 줄은 아래.
- **🔴 queue·dynamic 배열의 인덱스에는 봉인이 아예 없다 — 상수도 넷도**(pre-existing·오라클 ✓·§4.5.324 적대 리뷰 발굴). 256엔트리 `int q[$]` 에서 `q[-8'sd1]` 과 `q[s8]`(s8 = −1)이 **진단 없이** 원소 255 를 읽는다(iverilog 는 기본값 `0`). `int d[]` 도 같다. **쓰기 쪽은 loud**(W4020)라 read/write 비대칭이고, `dynarr.rs` 가 인덱스를 직접 낮춰 `seal_index_unsigned` 를 아예 안 부른다 — 고정 배열의 carve-out 보다 **넓은** 클래스다. ⚠️ verilator 는 2의 거듭제곱 크기에서 마스킹하므로 이 칸의 오라클이 아니다.
- **🔴 함수 호출 인덱스는 어느 봉인에도 못 온다**(pre-existing·오라클 ✓·§4.5.324 발굴). `arr[fneg(0)]`(`fneg` 가 `-8'sd1` 반환)이 **조용히** 원소 255 를 읽는다(iverilog `xx`) — 런타임 봉인이 `Call` 을 **반복 가능성**에서 거절하기 때문(부호 fill 이 두 번 부른다). §4.5.320/321 의 같은 뿌리.
- **🔴 `$bits` 를 상수 인덱스 식에 쓰면 `x` 를 읽는다**(pre-existing·오라클 ✓·§4.5.324 발굴). `m[$bits(m[0]) - 4'sd9]` 가 vita `xx` / iverilog `10` — 같은 값의 리터럴(`m[8 - 4'sd9]`)과 `$clog2` 철자는 정상이라 **`$bits` 한정**이다.
- **🔴 평범한 벡터의 비트/부분선택은 음수 상수 인덱스를 무부호로 읽는다**(pre-existing·오라클 ✓). `logic [15:0] pv = 16'hFFFF` 에서 `pv[-2'sd1]` 이 `01`·`pv[3'sd7 -: 2]` 가 `3` / iverilog `0X`·`x`. 별도 퍼널(`sealed_signed_index`/`norm_sub_k`)이라 §4.5.324 가 안 닿는다.
  - **🔴 인라인 바인드의 폭 결정이 `ir_bits_of` 의 지어진 폭을 믿는다**(pre-existing·§4.5.323 실측). 클래스 필드는 핸들넷의 32 를 답하므로 절단/확장 판정이 뒤집힌다 — `function [63:0] i16(input [15:0] x); i16 = x;` 에 `i16(c.bu)`(8비트 필드)가 `xxc3`. 창은 `필드폭 < formal폭 < 32`. 정본은 `canonical_self_width`.
  - **🔴 `real` rhs 는 §10.7 을 통째로 건너뛴다** — `resize_inline_assign` 의 `expr_is_real` early-return 이라 선언 폭이 절단을 못 한다(`f = r + x*x` 가 `013b` / iverilog `3b`). 봉인이 못 막는 유일한 문.
  - **🔴 `!trusted_w` 카브아웃 아래는 아직 샌다**(설계상 트레이드). 클래스 필드 rhs 가 지어진 32 == 선언 폭이면 같은-폭 팔이 맨 노드를 돌려준다 — `function [31:0] fh; fh = c.big + 1'b1;`(40비트 필드)가 대상 폭에 따라 값이 달라진다(`00000000` vs `0000010000000000`). 무조건 봉인하면 이 칸은 낫지만 위 첫 항목의 판별 칸이 깨진다 = **지어진 폭을 믿을 수 없다는 것의 대가**.
  - 곁: **actual 이 formal 보다 넓으면 바인드에서 절단하지 않는다**(의도적·`inline_fn.rs`) — `function [7:0] f(input [3:0] x); f = x;` 에 `f(8'hFF)` 가 `ff` / iverilog `0f` · **함수 반환을 replication count 로** 쓰면 `{f(8'h02){1'b1}}` 가 0 / iverilog `f`.
- **🔴 캐스트/인라인의 확장 부호가 미러에서 온다 — 클래스 필드는 순수한데도 못 받는다**(pre-existing·PRE==POST·§4.5.321 적대 리뷰 발굴). §4.5.320 은 넓히기 arm 의 정본 부호 채택을 **불순한 피연산자(프레임 `Expr::Call`)** 때문에 보류했는데, **signed 클래스 필드**도 그 arm 에 도달하고 그것은 **순수·반복 가능**하다 — `function signed [63:0] fw; fw = c.sf;`(`sf = 8'hAB`)가 `00…ab` / hand-IEEE `ff…ab`. 즉 정본 부호 채택은 **반복 가능성 술어로 게이트한 진짜 수정**이지 무가치가 아니다(전용 슬라이스).
- **🔴 캐스트의 문맥폭이 안쪽 자기결정 노드에서 멈춘다**(pre-existing·PRE==POST·두 오라클 ✓·§4.5.320 발굴). `64'(-16'(u16))` 가 `…fffb` / 두 오라클 `…fffb` 가 아니라 vita `000000000000fffb`(오라클 `fffffffffffffffb`), `8'(s4 * 4'(s8))` 가 `…f9`(−7) / `…09`(+9), `16'(s8 + 4'(u8))` 가 `000c` / `010c`. 무중첩 대조군 `64'(-u16)` 은 정상이라 트리거는 **중첩 캐스트·`$signed`/`$unsigned` 노드**다. 10,368칸 중 143칸, 전부 signed 좌변 + 안쪽 캐스트가 더 좁을 때.
- **캐스트 결과가 함수/태스크 formal 에 바인딩되면 부호를 잃는다**(pre-existing·오라클 ✓·§4.5.320 발굴). `function [31:0] gu(input [31:0] x)` 에서 `gu(16'(sa))` 가 vita `…0000fffd` / iverilog `…fffffffd` — 엔진 `eval_ctx` 의 `Call` 팔이 actual 을 **formal 의 부호**로 평가한다(§13.4.3 은 대입 문맥이므로 rhs 의 부호가 확장을 정해야 한다).
- **넓히는 캐스트가 불순한 피연산자의 부호 수정을 못 받는다**(§4.5.320 이 의도적으로 남긴 값). `extend_to` 의 부호 fill 이 피연산자를 두 번 부르므로 `16'(f())`·`int'(f())` 는 PRE 의 무부호 답을 유지한다(오라클 `fffd`/`fffffffd`). 닫으려면 **피연산자를 한 번만 부르는 4-state 보존 확장** 또는 callee 순수성 술어가 필요하다. 곁: `int'`/`byte'`/`shortint'`/`longint'` 는 `coerce_two_state` 때문에 피연산자를 **32/8/16/64회** 평가한다(PRE 동일).
- **캐스트가 원소의 부호를 청구하지 못하는 나머지 철자들**(전부 pre-existing·PRE==POST·§4.5.320 라운드 4 실측). `unpacked_elem_signed` 는 base 가 **단일 세그먼트 ident** 인 경우만 청구하므로 아래가 남는다 — 전부 `40'(x[0]*1)` 형태에서 vita `00000000fd` / iverilog `fffffffffd`: 다차원 `g[i][j]` · **패키지 한정 `pk::pm[0]`** · 프레임 로컬 배열 · dyn 배열/queue 원소(`net_is_static_array` 가 의도적으로 제외) · 인터페이스 배열 원소(같은 인터페이스의 **스칼라** `ii.v` 는 정상). ⚠️ **패키지 철자가 가장 급하다** — `arrays.rs` 는 `pkg::arr[i]` 를 원소로 라우팅하는 전용 arm 을 이미 갖고 있으므로 **분류기가 자기 lowering 리졸버와 어긋나 있고**, 그 결과 한 설계 안에서 같은 배열의 두 철자가 다른 값을 낸다(`pm[0]` 은 정답 · `pk::pm[0]` 은 오답). PRE 는 둘 다 틀렸으므로 회귀는 아니지만 분기 자체가 신호다. 곁: **계층 배열 원소는 캐스트가 `x` 를 주입한다** — `u1.sarr[0]` 무캐스트 읽기는 `fffffffffffffff9` 로 정확한데 `16'(u1.sarr[0])` 이 `000000000000xxf9`(PRE 동일·§4.5.320 이 비트 패턴만 64비트 x → 16비트 x 로 바꿨다).
- **🔴 fill override 가 타깃 폭이 아니라 32비트로 접히는 자리 셋**(pre-existing·PRE==POST·오라클 ✓·§4.5.314 적대 2렌즈 발굴). §4.5.314 는 `'0`/`'1` 을 **선언 폭에서 다시 접도록** 퍼널을 세웠고 그 폭이 존재하는 곳은 전부 고쳤으나, `param_decl_width` 가 `None` 을 내는 세 형태는 여전히 부모 쪽 32비트 fold 로 떨어진다 — ⓐ **>64비트**(`parameter [127:0] K` + `'1` → 하위 64비트만 `0000…ffffffffffffffff`, iverilog 는 128비트 전부 1). 이것은 "wide 파라미터의 OVERRIDE 는 loud" 라는 아래 §3 불변식의 **구멍**이기도 하다(같은 선언에 명시 리터럴 `128'hFF…` 를 주면 loud 인데 `'1` 은 조용히 통과) ⓑ **`time`**(64비트 모델이 아니라 32비트 — `#(.T('1))` 이 4294967295, iverilog 18446744073709551615) ⓓ **`real`**(`#(.R('1))` 와 `-G R='1` 이 둘 다 `4294967295.0` 를 설치한다 — iverilog `1.0` · 두 채널은 서로 일치) ⓒ **untyped**(IEEE §12.2.2 는 파라미터가 override 의 폭·부호를 **받는다**고 한다 — `parameter K = 3` + `#(.K(64'hDEADBEEF))` 가 vita −559038737 / iverilog 3735928559, 그리고 `'1` 은 iverilog 가 1비트로 봐 `1`). 셋은 한 뿌리(**타깃 타입의 폭을 정하는 규칙**)이므로 한 슬라이스로 다뤄야 하고, 세 채널(`#()`·`defparam`·`-G`)이 지금은 **서로 일치**한다(§4.5.314 가 맞춘 것) — 고칠 때 그 일치를 깨지 마라.
- **파라미터 선언 fold 가 네 벌**(pre-existing·PRE==POST·오라클 ✓·§4.5.314 적대 2렌즈 실측). 정본은 `params.rs::bind_one_param` 이고 나머지 셋이 각자 빠뜨린 것이 다르다 — `instance.rs` 의 모듈 본문 fold(override 처리 없음) · `generate.rs` · `package.rs`. 측정된 불일치: generate/package 는 `const_eval_in_scope` 로 접어 **fill 기본값을 선언 폭으로 안 접고**(`parameter [63:0] Q = '1` → `00000000ffffffff`), `package.rs` 는 **`param_range` 를 기록하지 않으며**(패키지 `parameter [15:8] P` 의 부분선택이 `x`) **`string`/`real` 을 라우팅하지 않는다**(자기 기본값만으로 E3009). §4.5.314 가 인터페이스 복사본(넷째)을 정본으로 흡수했으므로 남은 셋도 같은 방식이 가능하다 — **인스턴스가 아니라 CLASS 이므로 발견한 자리에서 고치지 말 것**(§4.5.311 교훈).
- **`--obs-dir` 의 run.json 이 `-G` 를 안 싣는다**(pre-existing·§4.5.314 발굴). `plusargs` 와 `source.blake3` 는 있는데 파라미터 override 는 없어서, `-G W=9` 와 `-G W=100` 으로 돌린 두 런의 run.json 이 타임스탬프 말고는 **동일**하다 — 효과가 **다른 설계**인 유일한 플래그에 대해 G2 관찰 rail 이 눈멀어 있다. §4.5.314 는 같은 논거로 `-v` echo 에 행을 넣었고 rail 에는 적용하지 않았다(§6 OBS-1 잔여와 같이 다룰 것).

**문서화된 divergence (수정 비대상·핀됨):**

- 크로스스코프 t0 decl-init race(양쪽 §6.8 합법·self-consistent) · 런타임 구성 `-0.0` 표시 · iverilog 자인 결함들(expression-force "evaluated once" 등).
- **`$stime` 의 부호 — 두 오라클이 갈리고 규격이 정했다**(§4.5.320). `16'($stime)` 이 t=0x8000 에서 vita/verilator 5.050 `00008000`, iverilog 13.0 `ffff8000`. IEEE 1364-2005 §17.7.2 = "returns an **unsigned** integer that is a 32-bit time". vita 는 캐스트 **밖에서도** 이미 무부호였고(`q = $stime` at t=2^31 → `0000000080000000`, PRE 동일) iverilog 만 signed `integer` 를 돌려준다 — PRE 가 캐스트 자리에서만 iverilog 와 맞았던 것은 자기모순이었다.
- **`#(.S("str"))` 가 적용되기 전에 W3056 을 한 번 낸다**(pre-existing·값은 정답): 부모 쪽 숫자 fold 가 먼저 실패해 "override 는 상수가 아니다; 기본값 유지" 를 찍고, 그 다음 string 채널이 정상 적용한다. 경고가 사실과 반대라 거슬리지만 값은 iverilog 와 일치한다.

## 3. Loud→supported 후보 (현재 전부 loud=안전 · additive)

> **✅ 외부 리포트 aes_top 2판 — 16항목 중 15 해결 (2026-08-07 · §4.5.313).** unpacked
> 배열 포트 · 선택적 import 의 패키지-스코프 해석 · `pkg::f()` 제약 완화 · 콤마 import ·
> 포트 연결의 사용자 함수 · `RPS'()` 폴딩 · string 파라미터 · 64비트 초과 파라미터 ·
> `-G`/`--param` 오버라이드 · E4002/W4029 분리 · `$sscanf` scanset · 선언 전 사용 loud ·
> W1018 부분 timescale. **남은 1건 = §3.1 DPI-C(영구 비목표)**.
>
> 그 과정에서 **리포트에 없던 silent-wrong 7건**과 **적대 2렌즈의 결함 17건**을 닫았고,
> 남은 잔여는 아래 두 줄이다:
>
> - **unpacked 배열 포트의 방향 불일치는 loud**(`[0:3]` ↔ `[3:0]`). IEEE 1800 §7.6 은
>   원소를 **위치로** 짝지으므로 flat-index 연결이 순서를 뒤집는다(vita 4 / iverilog 1 로
>   실측). 두 번째 대응 규칙을 짓는 대신 거절했다 — 구현하려면 위치↔인덱스 매핑을
>   `wire_array_port` 와 배열 대입 양쪽이 **한 철자**로 써야 한다.
> - **wide(>64비트) 파라미터의 OVERRIDE 는 loud**. 선언은 `wide_param_bits` 로 값을
>   지키지만 override 채널은 i64/string 뿐이라 `#(.K(128'h…))` 는 거절된다(값이 조용히
>   기본값으로 가지 않도록 loud 로 만든 결과). 캐리려면 `ResolvedOverride` 에 wide 슬롯이
>   필요하고, 자식의 선언 폭을 모르는 부모 스코프에서 접어야 하므로 `fill` 과 같은
>   "원문을 넘겨 자식 폭에서 재폴딩" 형태가 된다.

**§4.5.314 이 남긴 1건 (오라클 ✓ iverilog):**

- **`defparam` 이 INTERFACE 인스턴스에 안 닿는다** — `interface ifc; parameter D = 8; … endinterface` + `ifc a(); defparam a.D = 255;` 가 `W3056 … matched no instance` 를 내고 **기본값을 유지**한다(iverilog `d=ff`, vita `d=8`). PRE·POST 동일한 pre-existing 이고, 경고가 있으므로 silent 는 아니다. 원인 = `defparams` 소비가 `elaborate_instance` 에만 있고 `iface_inst.rs` 는 자기 `overrides` 만 본다 — 인터페이스 바인딩 루프가 §4.5.314 에서 정본 바인더를 쓰게 됐으므로 `defparams.remove(path)` 를 같은 자리에서 병합하면 된다.

**슬라이스 #8(§5.1-ae)이 남긴 1건 (오라클 ✓ iverilog):**

- **`$writemem*` 의 타깃이 서브루틴의 whole unpacked-array 로컬이면 E3009** — `task automatic t; reg [7:0] loc[0:1]; … $writememh("x.txt", loc);` 가 *"a whole unpacked-array formal has no value here"* 로 거부된다(iverilog 는 실행하고 파일을 쓴다). 두 백엔드 동일한 pre-existing honest-loud. ⚠️ **여는 슬라이스는 seam 도 함께 지어야 한다** — #8 의 `read_task_net` 은 이 거부를 **도달 불가 논거**로 삼아 아레나를 맨손으로 읽고(프레임·힙 넷은 그 store 가 소유하지 않으며 `assert_owns` 는 `debug_assert!` 다), 그 논거의 핀이 `writemem_targets_the_seam_cannot_own_are_refused_before_the_backend` 다.

**외부 round-28 이 남긴 4건 (§4.5.284 · 전부 실사용 ASIC 트리에서 실측된 사이트 · 오라클 ✓ iverilog):**

- **양 끝이 음수인 ASCENDING 팩트 범위가 폭 1 로 클램프**(`reg [-33:-2]` → `$bits` vita 1 / iverilog 32 · **loud**: `W3056`). 하강 쌍둥이 `[-2:-33]` 와 혼합 `[3:-2]` 는 정상이라 갭은 그 한 조합뿐. §4.5.308 differential 이 잔차 308행의 원인으로 실측했고, 그 행들은 폭이 틀려서지 정규화 때문이 아니다(`array_geom.rs` 의 `allow_neg_lsb` opt-in 경로).
- **`$value$plusargs` 의 `%0d` 등 폭-지정 스펙은 과잉거부**(§4.5.304 differential 기록): iverilog 는 `%0d` 를 받는다(값 5 기록), vita 는 E3009 loud. 스펙 파서가 폭 수식자를 벗기면 끝 — 단 `exec::plusargs::effect` 의 conv 문자 추출과 **한 철자**여야 한다(거부가 풀리면 `'0'` 이 %s 로 읽히는 함정이 정본 술어에 기록돼 있다).
- **static 함수의 로컬을 정의-대입 前에 읽으면 E3010**(§4.5.311 그라운딩 발굴·오라클 ✓ iverilog 는 X 로 실행): `function integer f(input integer x); integer s; begin s = s + x; f = s; end` 이 `undeclared net/variable top.s` 로 거부된다. 뿌리는 블록-로컬 flatten 모델(definite-assignment 를 요구)이고, **본문에 제어 흐름이 하나라도 있으면 프레임이 되어 정상 동작한다** — 즉 갭은 "직선 본문 + 읽고-쓰는 static 로컬" 조합뿐. (③층 S3a 가 이 비대칭에 걸려 static 슬랩 경로를 "도달 불가" 로 오판했다.)
- **프레임-로컬 배열의 범위 밖 읽기에 E4002 가 없다**(§4.5.311 differential 발굴·값은 iverilog 와 일치): 모듈 배열의 같은 접근은 E4002+exit 1 인데 프레임 로컬은 조용히 X 를 내고 exit 0. 프레임 로컬 배열은 **packed 슬롯**이라 array-word 개념이 없어서 진단이 구조적으로 안 붙는다 — elaborate 가 슬롯의 원래 기하를 남겨야 한다. vita 내부 비일관(모듈 vs 프레임)이 판정 근거다.
- **겹침 CA 의 E3001 일부는 iverilog 가 해상하는 형태**(§4.5.303 경계 실측): 같은-범위 part-select 쌍(`assign z8[3:0]=…` ×2)·delayed+plain 겹침을 iverilog 는 비트 단위 wire 해상으로 받는다(`zzzz0xx1`). vita 는 E3001 loud — 정직하나 미지원. 비트 단위 드라이버 맵(§2 ⓑ 와 같은 인프라)이 선결.
- **`specify … endspecify` 블록 수용** — `specparam` 은 §4.5.284 에서 모듈 레벨 상수로 받게 됐지만 **블록 자체는 아직 E2002** 다. 리포터의 우회는 이제 "블록을 지우되 `specparam` 은 키워드째 살린다"로 단순해졌으나, 벤더 모델(EFUSE·표준셀)이 `specify` 를 갖고 오는 것은 흔하므로 스크립트 없이 받는 것이 목표. **범위** = 블록 안의 `specparam` 을 모듈 아이템으로 hoist + 경로지연/타이밍체크는 버린다. ⚠️ **선행 결정 하나**: 버린다는 사실을 말할 창구가 없다 — 파서에 경고 채널이 없고(`hdl_parser::parse` 는 에러 채널만), 조용히 버리면 `$setup` 등이 **loud→silent 하강**이다. `ModuleItem` 에 마커 variant 를 두고 elaborate 가 `W3056` 을 내는 것이 현재 후보(`.vu` 해시 re-pin, format 불변).
- **이벤트 컨트롤의 계층 참조 실지원** — `always @(`TOP.a_uVDC.RTRIM_I)`(파운드리 ADC 모델). **읽기는 이미 동작**하므로 sensitivity 등록만의 문제다. §4.5.284 는 진단만 정확하게 했다(EVENT CONTROL 이라고 말하고 동작하는 우회를 제시). **범위** = `deferred_hier` 와 같은 형태의 지연 해소가 필요한데, 패치 대상이 `ir::Expr::Signal` 의 net 슬롯이 아니라 **`Process.sensitivity.edges[i].net`** 이고 그 프로세스는 아직 push 되지 않았다 → (proc_idx, edge_idx) 를 예약해 두고 instance 확정 후 패치하는 새 lane.
- **cross-process `disable`** — 파운드리 EFUSE 모델의 방어적 관용구. **리포터가 값싼 경로를 지목했다**: 대상 블록에 지연문이 없어 `disable` 실행 시점엔 이미 완료 상태이므로 **실질 no-op** 이다. 즉 *"현재 suspend 상태가 아닌 대상에 대한 `disable` 은 아무 일도 하지 않는다"* 만 구현해도 이 라이브러리가 통과한다 — 전체 cross-process 의미(suspend 된 프로세스 강제 종료)를 다 짓지 않아도 된다. ⚠️ **경계 주의**: "아무 일도 하지 않는다"가 **suspend 중인 대상까지 조용히 무시**하면 silent-wrong 이다. 대상이 활성이면 여전히 loud 여야 한다.
- **`E3010`/`E3009` 의 file:line 이 일관되지 않다** — 붙는 자리도 있고(`d_trunc.v:3:20`) 계층 경로만 나오는 자리도 있다. 다른 코드(`E2002` 등)는 `file:line:col` 이 정확히 붙어 대비된다. 리포터는 **모듈을 하나씩 elaborate 하는 이분탐색**으로 사이트를 찾아야 했다. §4.5.249 의 `diag::SpanResolver` 가 이미 있으므로 **앵커를 안 넘기는 호출 지점 전수**가 범위.


**~~frame-body validator over-scan~~ RESOLVED §4.5.172** (pre-existing false-REJECT · §4.5.171 적대 agent 발굴 · V5 무관): `classify_frame_body`의 linear `block_base..func_blocks.len()` 스캔이 **POST-PASS**(`resolve_frame_task_rejects`·전 func lower 후 subset task 검증)서 `func_blocks.len()`=**전체** 끝이라 **뒤에 정의된 func 블록까지 over-scan**→그들의 (합법적으로) out-of-frame인 output-formal write를 자기 것으로 오판→subset task를 E3009("assignment to a net outside the function")로 false-reject(iverilog는 accept). **fix = reachable-block CFG walk**: entry(`self.funcs[fid].entry`)서 자기 CFG 엣지(`Goto`/`Branch`/`Call`→`ret_bb`[callee entry 아님]/`Delay`·`Wait`→`resume`)만 순회→다른 func·dead 블록 미방문. **correct-or-loud**: 미방문 블록은 타 func이거나 dead(실행 안 됨)이므로 verdict는 false-reject만 DROP(silent-wrong 불가). `frame_task_pending` 튜플서 dead `base` 필드 제거. 적대 differential 全 MATCH(nested call·if/case/for·suspendable-mixed·5-task interleave·still-loud=모듈 net write). 신규 `frame_subset_overscan.rs`×4. 상세=ARCHIVE §4.5.172.

**round-20 이 남긴 2-세그먼트 호출의 정밀도 손실(오라클 ✓ · 의도된 사다리 교환):**

- **output-formal 호출 왼쪽의 `c.m()`·`pkg::f()` 가 loud** — `order_walk` 의 opacity 는 "이 호출이 mutated root 를 읽을 수 있나"를 물어야 하는데, 2-세그먼트 호출은 `callee_body_cannot_touch`(단일 세그먼트 전용)로 답할 수 없다. §4.5.276 은 **내장 컨테이너/문자열 메서드만** 적극 식별해 통과시키고 나머지는 loud 로 잘랐다. 그래서 **자기 멤버만 읽는 클래스 메서드**(`class C; int k; function int get(); return k;` → PRE `q=11` 정답) · **자식 인스턴스 함수**(`u.size()`) · **모듈 self-path 함수**(`t.size()`) · **covergroup 메서드**(`ci.get_coverage()`)가 정직한 loud 가 됐다. **`pkg::f()` 는 복구했다** — `inline_pkg_function` 이 self-contained·straight-line 본문만 받으므로 의무가 상류에서 이미 이행돼 있었다(§4.5.276 6라운드). **이것은 교환이다**: PRE 는 2-세그먼트 callee 본문이 root 를 읽으면 **무조건 조용히 틀렸고**(계층 self-path `t.o` 로 `q=12`, 정답 11 — iverilog 확인), 그 본문을 여기서 벳할 방법이 없었다. correct-support = 클래스 메서드/패키지 함수 본문을 해소하는 리졸버(§4.5.275 후속의 `c.m(x) + f(x)` 와 같은 전제) → 그 슬라이스에 묶는다.

**round-20 에서 실측한 dyn-array-formal 위치 갭 — 두 hoister 의 위치 집합 통일(오라클 ✓ iverilog 9/10):**

- **dyn-formal 호출이 못 가는 자리 = `&&`/`||` 우변 · 다른 호출의 인자 · select/lvalue 인덱스 · `case` scrutinee · `repeat` 카운트 · cast/replicate 피연산자.** 17-위치 행렬 실측(§4.5.276): 지원 7(직접 rhs·concat·비교·systask 인자·`?:` arm·NBA rhs·`return`) / loud 10(그중 **9건은 iverilog PASS = false-loud** · 나머지 1건 `case` scrutinee 는 iverilog 가 `string` case 를 거부해 무오라클 → hand-IEEE · **wrong 0**). 근인은 §3.1 과 **같은 뿌리**다 — dyn 용 좁은 hoister(`has_unhoistable_dyn_formal_call`/`hoist_dyn_formal_calls`)와 §4.5.275 의 범용 hoister(`shape()` 기반)가 **서로 다른 위치 집합**을 갖는다. correct-support = 범용 hoister 가 dyn-formal 호출도 흡수(leaf 에서 copy-out 대신 `__t = f(arr)` 를 `lower_stmt` 로 방출; dyn callee 는 input formal 만 가지므로 순수해서 eval-order 수리가 불필요). **착수 시 함정(§4.5.278 로 절반 완화)**: `hoist_stmt_general` 의 stand-down 은 이제 `in_frame_body()` 가 아니라 `frame_fn_lowering` 이다 — 프레임 **태스크** 본문은 hoist 하므로 그 절반의 함정은 사라졌다. 남은 것은 프레임 **함수** 본문뿐이고, 거기서는 `dyn-formal hoist 가 지원되는 것이 정상`(Family C·r17)이라 stand-down 을 statement 단위가 아니라 **call-kind 단위**로 쪼개야 한다. 그래서 별도 슬라이스로 뒀다.

**round-19 에서 loud 로 올린 silent-wrong 2건 — correct-support 후속(오라클 ✓):**

- ~~**frame body 안의 파일 읽기**(R19-X2)~~ **RESOLVED(§4.5.277·상세=ARCHIVE)** — 답은 `&self` 실행기에 파일 상태를 interior-mutable 로 여는 것이 아니라, **그 서브루틴을 `&mut` 실행기로 라우팅**하는 것이었다(분류기가 rhs 의 문장 수준 효과를 보게). **잔여** = 직접-rhs 가 아닌 자리의 파일 읽기 — `return $fgets(line, fd);` 는 정직한 loud(E3009, "…only as the direct rhs of a blocking assignment"). 라우팅 술어와 로워링이 **같은 형태**(직접 rhs)에 키잉되어 있기 때문이고, 넓히려면 둘을 함께 넓혀야 한다.
- **package 함수의 default 인자 스코프**(R19-X1 잔여). §4.5.274 의 `default_binding_matches_decl_scope` 는 `tf_decl_scope`(= func_table 을 채운 인스턴스 프리픽스)와 비교하는데, package 함수는 **import 한 모듈**의 프리픽스로 기록된다 — IEEE 는 package 스코프에서 평가한다. 가드는 개선이지 완결이 아니다. correct-support = 심볼별 선언 스코프 기록.

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

**외부 round-20 리포트(2026-07-27) = 12 가족 중 10 RESOLVED**(§4.5.229 part-select 바운드 · §4.5.234 enum formal `.name()` · §4.5.248 8 가족 · §4.5.249 §6 진단 위치). **잔여** = ① §4.11 의 미격리 79 건(리포터의 file:line 격리 대기 — 다만 **round-16 의 §3.4 가 그 가족의 지배적 형태였다**: 두 중첩 레벨 재사용은 §4.5.268 에서 correct-support) ~~② 같은-이름 가족의 마지막 2형~~ **부분 RESOLVED**(§4.5.268) — 한 블록이 다른 블록을 **감싸는** 쌍은 shadowing 이라 여전히 loud(별개 규칙), 모듈 넷과 이름이 겹치는 블록 로컬도 그대로. **두 중첩 레벨의 형제 트리**만 열렸다 ③ `$sformatf` 를 **ternary arm / 단락 우변 / `$monitor`·`$strobe` 인자 / 태스크 인자**에 두는 형태. ③의 뿌리는 하나로 좁혀졌다 — `eval` 의 `SysFuncId::Sformatf` arm 이 포맷 문자열을 무시한다(§4.5.252). `format_args_str`/`render_template` 을 `&SimState` 가 아니라 리더 제네릭으로 올려 `EvalCtx` 에서 쓸 수 있게 하면 ③이 통째로 닫히고 statement-level hoist 를 은퇴시킬 수 있다 — 리포트 자신의 16-블록 충실 복제본도 재현 못 했고 이 체크아웃에서는 볼 수 없다. §4.5.249 의 file:line 이 리포터가 좁힐 수단이다.

**외부 round-16 리포트(2026-07-29·`6b6b8ef` 기준) = 뿌리 7 + 진단 품질 4, 전부 RESOLVED**(§4.5.266/267/268). 리포트가 센 진단 85 건 중 오진이 62 건이었고, 근인별로는 §3.1 definite-assignment 의 제어-흐름 격자(49) · §3.4 두 단계의 스코프 중첩 불일치(17+1) · §3.5 declarator 단위가 아닌 선언 단위 거부(9) · §3.2 문장 위치 호출(4) · §3.3 고정 `automatic` 배열(2) · §3.6 SoA record queue 의 discarding pop(2) · §3.7 `return f(dyn)`(1) 이다.

**잔여 = 없다.** 다만 이 라운드가 **의도적으로 loud 로 남긴** 것 3가지는 갭이 아니라 규칙이다: ① 원소별로 채운 고정 배열에서 **커버리지를 증명할 수 없는** 형태(계산 인덱스·조건부 쓰기·불완전 집합) ② 한 프레임 본문 표현식 안에서 **같은 dyn-formal 함수를 2회** 부르거나 **자기 재귀**로 부르는 형태(마커 슬롯이 하나) ③ 한 블록이 다른 블록을 **감싸면서** 같은 이름을 재선언하는 shadowing(§3.4 가 연 것은 형제 트리다). 셋 다 진단이 그 이유를 직접 말한다.

**§3.8(리포트의 "미분류 1건")도 §3.4 와 같은 뿌리였다** — 구조체 로컬이 멤버별 넷으로 분해되면서 `automatic` 플래그를 잃어 STATIC coalesce 분기의 다른 문구를 달고 나왔을 뿐이고, 두 레벨 형태에서 스코핑을 잃은 것이 정확히 그 멤버들이었다(6b6b8ef 에서 그 문구 그대로 재현·현재 동작). 같은 문구를 다는 이웃 형태 하나는 **loud 로 남아야 한다** — 초기화자를 가진 static 블록 로컬 둘이 한 이름이면 평탄화된 넷 하나에 pre-arm 초기화가 둘 걸려 뒤가 앞을 덮으므로, 받아들이면 앞 블록이 뒤 블록의 값을 읽는다(iverilog 7/9 → 9/9). 둘 다 핀.

**리포트가 오라클로 쓴 iverilog 13.0 의 결함 2건**(vita 가 IEEE 정답): ① 루프 본문 블록이 **로컬을 선언하면** `break` 가 `continue` 처럼 동작한다(`for (i<4) begin int L; if (i==2) break; … end` → iverilog 가 i=3 을 계속 돈다; 로컬이 없으면 iverilog 도 정확히 멈춘다 — vita 는 PRE/POST 동일하므로 이 라운드와 무관하다). ② `case` item 안의 `continue` 에서 `vthread.cc` assertion 으로 abort 한다(abort 전 출력은 vita 와 일치). 둘 다 회귀 테스트 주석에 기록.

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

## 5. perf / 하드닝 — ★ **T0~T4 가 최우선 (2026-08-03 오너 지시)**, 나머지는 보류 판정

**⚠️ 하드닝 1건 — 프로세스 수준 메모리 가드가 없다 (슬라이스 #8 실측 · 오너 승인 2026-08-15).** 폭주한
`vita` 하나가 **33 GB × 2** 를 잡아 32 GB 머신을 jetsam → WindowServer 크래시 → **userspace watchdog
커널 패닉**까지 몰고 갔다(2026-08-14). 기존 가드(`max_deltas`·`max_body_steps`·`time_limit`)는 **델타도
문장도 진행하지 않는 루프**(시스템태스크 내부)를 구조적으로 못 본다. 이번 슬라이스는 **그 루프를
카운트 기반으로** 바꿔 실측된 형태를 닫았고(`$writemem*`), **일반 가드는 남는다**. 설계 제약 둘을 먼저
정해야 한다: ⓐ **할당 카운팅 전역 allocator** 는 매 할당에 원자연산 둘을 더한다 — 성능 축이 몇 주에 걸쳐
지운 바로 그 비용이라 기본 ON 은 회귀다 · ⓑ **RSS 샘플링 워치독 스레드**(초당 1회)는 핫패스 비용이 0
이지만 macOS 에서 `mach_task_basic_info` = **unsafe FFI** 라 unsafe 정책의 지정 모듈 확대가 선결이다
(Linux 는 `/proc/self/statm` 로 safe). ⇒ ⓑ + 기본 상한(물리 RAM 의 1/4 등) + `--max-mem` 이 현재 후보.

### 5.0 ★★ ③층 — 성능 축 **수확 체감 도달** · 다음은 **커버리지**(§5.1)

| 단계 | 상태 |
|---|---|
| T0 · S0 · S1(전부) | ✅ |
| **S2** 폭별 특수화 | ✅ **닫힘**(§4.5.329, 프로파일 근거) |
| **S3a** 호출 흡수 | ✅ |
| **S3** 바디 코드젠 | ✅ 슬라이스 1~2 · ⛔ **cranelift 절은 census 가 반증**(§4.5.334) |
| **S4** 스케줄 소거 | ⛔ **중단 판정 발동 — 이득 <1.3×**(§4.5.335) · 스케줄러 유지 |
| **S5** NBA 전용화 | ⛔ 같은 이유로 보류 — 표적이 `k_schedule_nba_scalar` **3.8%** 뿐 |
| **S6** 3-OS 결정성 | 코드젠이 없으므로 **불필요**(현 native 는 arch-무관 Rust) |

**picorv32 native/vm = 1.73×**(0.504 vs 0.876) · 고정 비용 19 ms(3.7%) → **상한 26.8×**
(⚠️ doc-21 §6.3 의 *"~85 ms · 14% · 상한 7×"* 는 낡았다 — 이 실측으로 정정).

> **⭐⭐ 왜 여기서 멈추는가 — 프로파일이 세 단계를 연달아 닫았다.**
> S3 의 cranelift 논거는 *"leaf load 와 산술을 인라인한다"* 인데 실행 census 는 **산술 1.5% ·
> `Load` 39.3%(이미 직접 접근) · 인라인 불가한 30.8% 가 하필 이 저장소가 한 철자로 통합한 4-state
> 규칙 함수들**이었다(§4.5.334). S4 의 표적(`propagate` 2.1 + `wake` 1.4 + `pass` 구성 ~2)은
> **≈6% = 1.06×** 로 자기 중단 판정(<1.3×) 아래다(§4.5.335). S5 도 3.8% 다.
> **1.73× 와 26.8× 사이는 vita 가 일부러 안 하는 것**(2-state 좁히기 · levelize)과 **없는 축**
> (멀티코어)이 메운다 — VCS·Xcelium·verilator 가 하는 일이 그것이다. 셋 다 **정확성 계약을 거래**
> 하거나 **아키텍처를 바꾸는** 일이라 오너 판정 사안이다.

### 5.1 ✅ **완료** — 실행기 둘로(interp = 오라클 · native = 제품) · 오너 지시 2026-08-10

> ⭐ **Phase A~D 가 모두 끝났다. 다음에 무엇을 할지는 [§5.2 재개 지점](#★★★★-52-재개-지점--세션이-끊겼다면-여기부터-2026-08-17) 을 보라.** 아래는 그 과정의 슬라이스 기록이다.

**목표 형태**: `--backend interp`(테스트 전용 오라클) + `--backend native`(제품). **②층 VM 은퇴.**

**왜**: ⓑ(코드젠)가 언젠가 필요해지면 비싼 것은 cranelift 가 아니라 **4-state 규칙이 두 철자가 되는
것**이다. 실행기가 둘이면 그 긴장이 *"두 번째 철자를 만드는 일"* 에서 *"오라클이 지키는 코드
생성기를 만드는 일"* 로 바뀐다.

---

## ★★★★ 5.2 **재개 지점 — 세션이 끊겼다면 여기부터** (2026-08-17)

> **Phase A · B · C · D 가 전부 끝났다.** 아래는 *"다음에 무엇을 해야 하나"* 의 정본이고,
> 이 절만 읽으면 재개할 수 있게 적었다.

### 지금 상태 (한 화면)

| | |
|---|---|
| 기본 백엔드 | **`native`**(③층) · 코퍼스 **100.00%** 실행 · 발산 0 |
| 제품 형태 | `--no-default-features` = **실행기 하나** · 게이트 거부는 **치명** |
| 성능 | 벤치 **8/8 에서 native < vm** · picorv32 native/vm **0.60** |
| 코드젠 | **기본 OFF · 기각됨**(§5.1-be) — 빌드·배선·측정·정확성은 전부 갖춰 둔 상태 |
| 게이트 | **5,477 tests green** · no-oracle lib green · clippy **3 구성** 0 · fmt 0 · format_version **26** · MsgCode **65** |

### 다음 후보 — 우선순위 순

| 순위 | 트랙 | 왜 여기 | 착수 조건 / 첫 걸음 |
|---|---|---|---|
| **1** | **정확성 큐 — §2 silent-wrong 잔여** | 이 저장소의 **최상위 원칙**이 정확성이고, 성능 축은 방금 수확 체감에 도달했다 | §2 를 위에서부터 **하나씩 재현** → 오라클(iverilog/verilator)이 답하면 수정, 아니면 hand-IEEE |
| **2** | **§3 loud → correct-support 승격** | 오늘 loud 인 것은 **안전하지만 기능 갭**이다. 사다리를 올리는 유일한 방향 | §3 표에서 **오라클이 답하는 행**부터. ⚠️ *"오라클이 없다"* 는 미루는 이유가 **아니다**(memory: no-oracle-not-a-defer-reason) |
| **3** | **§6 G2 OBS 잔여** | 최종목표 G2 축이고 정확성과 **직교**라 병렬 가능 | SPEC = [preview/19](preview/19-ai-agent-observability.md) · 남은 항목은 §6 표 |
| **4** | **성능 — 스케줄러 축**(✅ **측정됨 2026-08-18**) | picorv32 프로파일: **스케줄러 29.0%** self + 그것이 유발하는 **할당 5.8%** ≈ **35%** 가 아무도 안 건드린 축이다(표현식 41.5% = Phase D 의 영역 · 쓰기 17.1%) | ⭐ 첫 표적이 **D6 과 같은 모양으로** 보인다 — `propagate` 가 **델타마다 `Vec` 셋**(`changed`·`woken`·`clocked`)을 새로 만들고 그것만 **3.3%**. 그다음 = `settle_cont_assigns` 9.2 · `dispatch_body` 7.8 · `k_schedule_nba_scalar` 3.9 |
| **5** | ⛔ **D2-b(저장소 2-state)** | **거부됨** — 트랩이 사다리 하강이다 | 재개하려면 **정확성 거래 없는 방법**을 먼저 찾아야 한다 |
| **6** | ⛔ **코드젠 재착수** | **기각됨** — 경계가 ~38%, 천장이 11% | 재개 조건 **하나**: leaf 로드와 2-state 산술을 **생성 코드 안에 인라인**(호출 0)하고 **의미를 두 번 적지 않을** 방법 |

### 스케줄러 축 프로파일 (2026-08-18 · 측정 트리거 실행 결과)

**방법**: release + 심볼(`CARGO_PROFILE_RELEASE_{STRIP=none,DEBUG=1}`) · picorv32 를 `-G CYCLES=1000000`
으로 **12.6 s** 워크로드로 확대 · `/usr/bin/sample vita 10 -wait` · self time · 작업 샘플 6,929
(대기 스레드 제외). ⚠️ **첫 실행은 버려라** — 콜드 캐시가 native 를 1.04 s 로 보이게 했다(안정값 0.51).
재현된 비율: native **0.51** / vm 0.88 / interp 1.39 ⇒ **native/vm 0.58**(기록된 0.60 과 일치).

| 층 | self | 비고 |
|---|---:|---|
| 표현식 평가 | **41.5%** | Phase D 가 작업한 축(`exec_vm::run` 11.8 · `WProg::run` 10.5) |
| **스케줄러** | **29.0%** | **미최적화** — `settle_cont_assigns` 9.2 · `dispatch_body` 7.8 · `k_schedule_nba_scalar` 3.9 · `simulate` 3.7 · `propagate` 2.3 · `wake` 1.6 |
| 쓰기 퍼널 | 17.1% | `write_lvalue_inner` 6.8 · `write_chunk_word` 5.4 · `write_routed` 3.2 |
| 할당자 | 9.3% | **호출자가 스케줄러 쪽이다**: `propagate` 3.3 · `eval_ctx` 2.9 · `simulate` 0.7 · `note_change` 0.7 · `wake` 0.6 |
| 진단 | 1.3% | `drain_range_diags` |

⭐⭐ **코드젠 재착수의 측정 트리거는 발화하지 않았다.** 실제 설계에서도 op 디스패치가 앉은 두 함수
(`exec_vm::run` + `WProg::run`)의 **self 합이 22.3%** 이고 그중 제거 가능한 것은 디스패치 부분뿐이라
§5.1-be 가 잰 **천장 8.9~11.3%** 와 같은 자리다 — **거래는 여전히 11% 벌고 25~38% 내는 것**이다.

⭐⭐ **대신 프로파일이 더 큰 표적을 줬다.** 스케줄러 29% + 그 축의 할당 5.8% ≈ **35%** 가 미최적화이고,
첫 자리는 **D6 과 정확히 같은 모양**이다 — `native/run.rs::propagate` 가 **호출(=델타)마다**
`Vec::new()` 를 셋(`changed`·`woken`·`clocked`) 만든다. ⚠️ 다만 **크기는 아직 안 쟀다**: 이 3.3% 는
할당·해제 자체의 비용이지, 재사용으로 얼마가 회수되는지는 A/B 로 재야 한다(§5.2 착수 의례 3번).

### 착수 전 의례 (어느 트랙이든)

1. **census/프로파일을 다시 돌린다** — 앞 슬라이스가 게이트를 움직이면 다음 표적의 숫자도 움직인다(§5.1-p 가 정한 규칙이고 Phase D 에서 **네 번** 값을 냈다).
2. **[ENGINEERING_RULES](ENGINEERING_RULES.md) 를 읽는다** — Phase D 가 규칙 **여덟 개**를 추가했다.
3. 성능 슬라이스면 **A/B 를 두 번** 재고 **대조군**(그 코드를 안 부르는 형태)이 얼마나 움직이는지로 노이즈를 캘리브레이션한다 — ⚠️ 이 기계는 load average 가 5~7 일 때가 있고 그때 ±5% 가 뜬다.

---

## ★★★ 정본 실행 순서 Phase A~D — **오너 확정 2026-08-12. 이 순서대로 간다.**

⭐⭐ **기계어 코드젠은 목표다**(오너 판정 2026-08-12). §4.5.334 census 가 반증한 것은
*"오늘의 IR + `Value` 모델 위에 cranelift 를 얹으면 이긴다"* 이지 *"코드젠이 진다"* 가 아니다 —
그 census 의 **인라인 불가 30.8%**(`LogBin` 17.4 + `Unary1` 7.9 + `Cmp` 5.5)가 비싼 이유는
**함수 호출이라서가 아니라 4-state 라서**다(`log_bin_tri` 는 `Tri` 둘, `RedFacts::absorb` 는
`known0` 재마스크, `unary1_word` 는 real 가드). **2-state 로 좁혀지면 셋 다 기계어 한 명령이다.**
⇒ **P1(2-state 좁히기)이 코드젠의 본체이고 cranelift 는 마지막 단계다.**

| 선행조건 | 상태(2026-08-17 종료 시) |
|---|---|
| **P1** 2-state 좁히기 | ✅ **평가 축은 D2-a 가 했다**(동적 레인 · 정확성 거래 0) · ⛔ 저장소 축(D2-b)은 **거부** |
| **P2** 핫 경로에 `Value` 없음 | ✅ §4.5.331 + **§5.1-az**(leaf fast path) — ⚠️ 그런데 **JIT 경계가 그것을 되살린다**(`jit::mk` 12.4%) |
| **P3** **제품 실행기 하나** | ✅ Phase B/C |
| **P4** 스케줄러를 내부 루프에서 제거 | ✅ dirty-driven settle 이 **이미 배송돼 있었다**(D3) |

⚠️⚠️ **선행조건 넷이 다 충족된 상태에서 재측정한 결과가 §5.1-be 이고, 그래도 코드젠이 진다.**
*"2-state 로 좁혀지면 셋 다 기계어 한 명령"* 은 **참인데 그것이 표적이 아니었다** — 비용은 산술이
아니라 **경계**에 있다. ⭐ **다시 볼 조건은 하나**: leaf 로드와 2-state 산술을 **생성 코드 안에
인라인**할 것(호출 0). 그 **전제조건은 Phase D 가 방금 만들어 놨고**(leaf = 컴파일 시점 인덱스의 워드
둘 · 산술 = 평범한 정수 연산), 없는 것은 그것을 **두 번째 철자 없이** 하는 방법이다.

⚠️ **성능 중단 판정 하나를 철회한다** — S4(§4.5.335)의 ≈6% 는 **picorv32 한 설계**의 숫자다.
`levelize.rs` 가 이미 반례를 표로 갖고 있다: cont-assign 으로 체인된 깊이 1→24 에서
~~**7.8 ms → 814.4 ms(104×)**~~ — ⚠️⚠️ **이 인용은 틀렸고 D1 이 실측으로 잡았다(2026-08-17, §5.1-av):
814.4 ms 는 `✅ COMB-DEPTH 해결(2026-08-01)` 의 *before* 열이다.** dirty-settle 이 이미 배송돼
2차 항을 없앴고(깊이 24 에서 14.1×), 오늘 baseline 을 다시 재니 깊이 24 = **55.5 ms**(interp ·
기록된 after 57.9 와 일치)로 **세 백엔드 모두 선형**이다. **⇒ Phase D3 는 이미 끝나 있다.**

그래도 **"벤치가 한 설계면 중단 판정이 편향된다" 는 참이고, D1 이 그것을 훨씬 더 나쁜 형태로
확인했다** — 아래 §5.1-av.

### Phase A — V1 커버리지 완주 (78.73% → ~100%)

측정된 greedy 순서(§5.1-b). 각 슬라이스 게이트 = **VM 과 바이트 동일 + 절대 앵커**
(⚠️ 차분만으로는 부족 — 아래 §5.1-e).

| # | 슬라이스 | census | 상태 |
|---|---|---|---|
⚠️ **census 는 슬라이스마다 움직인다 — 아래 숫자는 #8 직후(2026-08-15) 실측이고, 착수 전에 다시 돌린다**
(§5.1-p 규칙). 그리고 **행이 아니라 집합으로** 센다: 한 설계를 얻으려면 그 설계를 막는 행을 **전부**
닫아야 한다.

| # | 슬라이스 | 잔여 census(집합) | 상태 |
|---|---|---|---|
| **A1** ✅ | `stmt_effect` — 전원 배선, 행이 비었다(§5.1-g~-l) | 0 | **완료 · 78.73% → 81.24%** |
| **A2** ✅ | `class`/OOP(§5.1-p) · CRV(§5.1-t) | 0 | **완료 · 88.55% → 94.15%** |
| **A7** ✅ | functional coverage(§5.1-r) | 0 | **완료** |
| **A5** ✅ | 거부 시스템태스크 — `$fclose`(§5.1-m) · postponed(§5.1-s) · file_directed(§5.1-aa) · **`$writemem*`(§5.1-ae)** | **1** | **완료** — 남은 `$dumpall`/`$dumpon` 은 **코퍼스 인구 0**(그 1 은 손으로 적은 테스트 설계) |
| **A8** ✅ | 꼬리 — handle_copy · deferred assert · clocking · force/release · final · stage | — | **완료**(§5.1-q·-w·-x·-y·-ab·-ac) |
| **A6** ✅ | `real` + `real-slot`(D+S 짝) — **행의 이름이 틀렸다**(§5.1-af) | 0 | **완료 · 96.35% → 97.47%** |
| **A3-ii-b** ✅ | 실제로 **park 하는** 프레임 — 없던 것은 실행기가 아니라 **스택의 수명**이었다(§5.1-ag) | 0 | **완료 · 97.47% → 98.09%** |
| **A4** ✅ | fork 가족 넷 — 프로세스 fork(§5.1-am) · fork-in-frame(§5.1-an) · `wait fork`(§5.1-ao) · `disable fork`(§5.1-ap) | 0 | **완료 · 98.29% → 100.00%** |
| 꼬리 ✅ | ~~`probe`~~ · ~~concat heap chunk~~ · ~~out-of-window write~~ · ~~`$dumpall`~~(§5.1-ah·-ai·-aj·-ak) | 0 | **완료** |

> ## ✅✅ **Phase A 완료 (2026-08-16 · §5.1-ap)**
>
> **코퍼스 6,470 / 6,470 = 100.00% native · 거부 0 · flip 발산 0 · 전 스위트 5,468 green.**
>
> 게이트의 세 층이 전부 비었다 — `gate_refused!` 매크로 사이트 **17 → 0**(매크로 삭제) ·
> `systask_refusal` 집합 **6→4→2→0** · 실행기 거부 표 **4→5→4→3→1→0** · design 행 도달 가능한 것 **0**.
> ⚠️ **검사를 지운 것이 아니다** — 세 함수와 소비자는 남아 있고 `_`-free match 가 새 종류를
> 강제한다. 핀하는 것은 *"지금 비어 있다"* 이고 `is_empty()` 단언으로 박혀 있다.
>
> **해설·용어·전체 서사(초보자용) = [study/02](study/02-v1-native-coverage.md).**
> 슬라이스별 상세 = §5.1-g ~ §5.1-ap.

### Phase B — V2 = **빌드 분리** (오너 제안 채택 · 옛 "VM 삭제" 를 대체)

⭐⭐ **삭제보다 엄밀히 낫다**: 되돌릴 수 있고, 오라클이 테스트에 영원히 남고, 무엇보다
**제품 빌드에서 게이트 거부가 loud 가 된다** — 오늘 `--backend native` 로 거부된 설계는
**조용히 VM 으로 떨어진다**(run.json 에 `native.refused` 가 실릴 뿐 진단도 exit code 도 없다.
`lib.rs` 의 `effective_backend` 를 볼 것). **폴백 대상이 컴파일되지 않은 빌드에서는 거부가
loud 일 수밖에 없다** = 정확도 사다리 상승이고, 삭제만으로는 안 생긴다(interp 로 떨어진다).

⚠️⚠️ **아래 표는 2026-08-16 에 재측정해 세 곳을 정정했다 — 원래 판은 Phase A 이전에 쓴 것이라
§4.5.333(S3, tier-3 이 `backend::vm_exec` 를 재사용)을 반영하지 못했다.**

| # | 무엇 | 게이트 |
|---|---|---|
| **B1** ✅ | 전 스위트 `--backend native` 초록 + **기본값 전환**(§5.1-aq) | **완료 2026-08-16** — 두 철자 전환 · 핀 3개 처리(둘 반전 · 하나 **재구조화**: 기본값이 native 면 옛 비교가 항진명제) · **양방향 flip**(native 5,469 통과 / vm 5,466+실패 3 · 갈리는 것은 정확히 같은 셋) |
| **B2′** ✅ | ⚠️⚠️ **표적이 틀렸다 — §5.1-b2 가 착수 전 측정으로 뒤집었다.** 삭제 가능량은 5,430줄이 아니라 **≈95줄**이고 대가는 **오라클+테스트 13파일**이다. 재정의: `Backend::{Interpreter,Bytecode}` 와 그 **디스패치**를 `oracle` feature 뒤로(삭제 0) | 게이트 대상 ≈150–250줄 |
| **B3′** ✅ | ⚠️⚠️ **`exec/` 는 인터프리터가 아니라 공유 의미 층이다**(§5.1-b2) — tier-3 이 49군데 쓰고 `exec/process.rs` 안에 **`compute_effect`/`apply_effect`** 가 있다. 모듈 단위로 감쌀 수 없고 **함수 단위 `#[cfg]`**(`run_process` 만). 재정의: CLI 의 `--backend interp\|vm` 철자를 같은 feature 뒤로 | 소수 |
| **B4a** ✅ | **교체를 경고로**(§5.1-ar) | **완료 2026-08-16** — `W4030 · W-RUN-BACKEND-FALLBACK` 신설(MsgCode 64→65) · 메시지가 **요청·실제·거부한 행**을 이름으로 부른다 · ⚠️ 발화 인구 0(fail-closed · 이빨은 손상 사이드카) |
| **B4b** ✅ | **`--no-default-features` 에서 치명으로**(§5.1-at) | **완료 2026-08-16** — ⚠️ **B2′ 가 연 구멍의 수정**(폴백 arm 을 gate 하자 판정 소비자가 사라져 거부 설계가 **그냥 돌았다** · 실측 exit 0·진단 0) · `fatal_run` 재사용(새 코드 0) · ⚠️ **래치만으론 부족**해 실행기 선택이 `st.finished` 를 먼저 묻는다(안 그러면 `expect` 패닉) · 제품 형태 lib 테스트 **147 green** |
| **B5** ✅ | CI 축 `build-no-oracle`(§5.1-as) | **완료 2026-08-16** — `-p cli -p sim-engine`(워크스페이스면 dev-dep 이 oracle 을 되살린다) · 스모크가 **설계 실행 + 없는 실행기 거부** 둘을 본다 |

#### 5.1-aq ✅ B1 — **기본 백엔드가 native 다** (2026-08-16)

⭐ **Phase B 의 첫 슬라이스이고 제품이 실제로 바뀌는 유일한 지점이다** — 이제 플래그 없이
`vita design.sv` 를 돌리면 ③층이 돈다.

**두 철자를 함께 뒤집었다** — enum 의 `#[default]` 와 `SimOpts::default()` 의 하드코딩된 리터럴.
⚠️ 그 둘이 **갈린 적이 있다**(§4.5.336: derive 만 뒤집었더니 CLI 절반만 움직여 census 가 틀렸다) ⇒
같은 자리에 주석으로 묶어 뒀고, `the_default_backend_is_native` 가 **둘 다** 단언한다.

⭐⭐ **근거는 두 측정이고 순서가 있다.** ⓐ **커버리지** — Phase A 가 도달 가능한 게이트 행을 전부
닫아 census 가 **6,470/6,470 · 거부 0** 이다. 즉 이 기본값이 **아무도 조용히 다른 실행기로 보내지
않는다**(거부가 남아 있었다면 그 설계들은 말없이 VM 으로 떨어졌을 것이다). ⓑ **등가, 그 다음에야
속도** — 전 스위트가 이 기본값으로 초록이고, 그것이 코퍼스 차분이 못 잡던 것을 잡아 온 게이트다
(§4.5.279 에서 코퍼스는 **18 타깃 39건**을 통과시켰고 스위트는 아니었다). 속도는 마지막 —
picorv32 release 번갈아 best-of-5: **interp 1.319 s / vm 0.838 s / native 0.513 s**(iverilog 13
0.585 s).

⭐⭐ **양방향 flip 을 돌렸고, 그것이 이 슬라이스의 진짜 주장이다.**

| 기본값 | 결과 |
|---|---|
| `Native`(배송) | **5,469 통과** |
| `Bytecode`(역flip) | 5,466 통과 · **실패 3** |

**두 방향에서 갈리는 것이 정확히 같은 세 테스트**다 — `the_default_backend_is_native` ·
`run_json_codegen_pins_the_vm_claim_and_reasons` ·
`run_json_codegen_is_backend_invariant_and_backend_is_recorded`. 나머지 5,466 은 **어느 쪽이든
바이트 동일**이고, 그것이 *"스위트가 native 전용이 되지 않았다"* 의 증명이다.

⚠️⚠️ **flip 런의 의미가 뒤집혔다.** B1 이전엔 *"native 가 기본값과 일치하는가"* 를 물었고, 이제
기본값이 native 이므로 **반대 방향으로 물어야** 한다 — 안 그러면 스위트가 조용히 native 전용이 되고
오라클이 더 이상 시험되지 않는다. 그 의무를 `Backend::Bytecode` 의 doc 에 적었다.

⚠️ **핀 하나는 반전이 아니라 재구조화였다** —
`run_json_codegen_is_backend_invariant_and_backend_is_recorded` 는 *"플래그 없는 실행(m1)"* 과
*"명시적 `--backend native`(m3)"* 를 비교하고 있었는데, 기본값이 native 가 되면 **그 비교가
항진명제**가 된다. VM 을 **명시적으로** 요청하는 실행을 넷째로 추가하고 바이트 비교를 그리로 옮겼다.

#### 5.1-ar ✅ B4a — **백엔드 교체가 조용하지 않다** (2026-08-16)

⭐ **판정은 늘 발행돼 있었지만 말해 주지는 않았다.** `run.json` 은 `backend_requested` 와 `backend`
를 나란히 싣고 `native.refused` 가 거부한 층을 지목한다 — 그런데 **찾아봐야 보이는 것은 아무도 안
찾는다.** §5.1-o 가 그 사고다: `--backend native` 로 돌린 설계가 사실 폴백돼 있었고 출력이 iverilog 와
정확히 일치해서 *"tier-3 이 동의한다"* 로 읽혔으며, `run.json` 을 열지 않았으면 그대로 배송됐다.

**지은 것 = 새 코드 `W4030 · W-RUN-BACKEND-FALLBACK` + `simulate` 의 스왑 지점 한 자리.**
메시지가 **요청·실제·거부한 행**을 전부 이름으로 부른다.

⚠️ **Warning 이지 Error 가 아니고, 이유는 정확도 사다리다**(§5.1-b1). 폴백은 **틀린 답이 아니라 느린
답**이다 — 백엔드 간 바이트 동일이 게이트이므로 VM 의 답이 곧 native 의 답이다. 기본 빌드에서
`exit≠0` 으로 만들면 **correct-support → loud** 로 내려간다. 승격은 폴백 대상이 **컴파일되지 않은**
빌드에서만(`--no-default-features` · **B4b**), 거기선 선택지가 loud 아니면 wrong 뿐이다.

⚠️⚠️ **오늘 발화 인구는 0 이고 그것을 숨기지 않았다.** Phase A 가 게이트 세 층의 도달 가능한 행을
전부 닫았으므로 **소스로는 이 경고를 만들 수 없다.** ⇒ **fail-closed 로 짓고 이빨은 손상된
사이드카**로 세웠다(`the_runtime_gate_is_exactly_design_and_storage` 가 STORAGE 층을 같은 이유로 같은
기법으로 건드린다). 새 게이트 행이 생기는 날 **아무도 기억하지 않아도** 이 경고가 그것을 말한다.

⭐ **테스트가 세 가지를 단언하고 셋째가 게으른 구현을 잡는다** — ⓐ 경고가 난다 ⓑ **거부한 행을
이름으로 부른다**(*"폴백했다"* 만 말하면 독자를 다시 `run.json` 으로 보내는 것이고, 그것이 이 경고가
없애려는 바로 그 문제다) ⓒ **깨끗한 대조군은 침묵한다**(안 그러면 모든 평범한 시뮬레이션이 이걸
달고 다닌다).

⚠️ **`simulate_capture` 로는 이 테스트를 쓸 수 없다** — 그 sink 는 `RtlOutput` 만 남겨 **모든 진단에
구조적으로 눈멀어 있다**(§4.5.298 이 기록한 축). 전용 sink 를 썼다.

⚠️ **bijection 게이트가 doc 을 강제했다** — MsgCode enum ↔ `preview/15` §0–9 가 1:1 이고 개수가
핀돼 있어(**64 → 65**), 코드를 더하면 문서 항목이 **함께** 들어간다. 좋은 강제다.

전 스위트 **5470 green**.

#### 5.1-b2 ⚠️⚠️ **Phase B 의 B2·B3 는 표적이 틀렸다 — 삭제할 것도, 감쌀 모듈도 없다** (2026-08-16 착수 전 측정)

**B2 착수 그라운딩이 계획을 뒤집었다.** 두 표적 모두 Phase A **이전에** 정해졌고, A 단계가 tier-3 을
공유 코드 위로 올리는 동안 그 전제가 통째로 무효가 됐다. **코드를 쓰기 전에 잰 것이 이 항목이다.**

**① B2("VM 삭제 5,430줄")** — `backend.rs` 의 pub 항목 **18개 중 죽는 것이 0개**다. tier-3 이
`is_codegen_able`·`compile_body`·`vm_exec`·`CompiledBody`·`CompiledBlock`·`CompiledTerm`·`Op`·
`CompileCtx`·`VmSlot`·`RegFile`·`OffFile` 를 `compiled_for`/`dispatch_body` 에서 쓰고(§4.5.333),
`codegen_coverage`/`codegen_report`/`native_eval_coverage*` 는 run.json 의 `codegen` 객체가 쓴다.
`native_eval/` 도 tier-3 의 `k_eval_native` 가 쓴다.

**실제 VM 전용 표면은 넷이고 합계 ≈ 95줄**이다 — `Scheduler::vm_run_body`(49) ·
`SimState::vm_compiled`(35) · `scan_arm` 의 `Backend::Bytecode` arm(7) · `vm_cache` 필드(4).
그 대가는 **오라클 하나와 테스트 13파일**이다(`Backend::Bytecode` 참조 45 · `"vm"` 28 —
`backend_equiv`·`dyn_storage`·`run_diagnostics`·`severity_tasks`·`end_to_end_b`·`perf_baseline` 등이
VM 을 **차분 상대**로 쓴다). ⇒ **비용/이득이 역전됐다.**

**② B3("`oracle` feature 로 `exec/`(3,246줄)를 감싼다")** — `exec/` 는 **인터프리터가 아니라 공유
의미 층**이다. tier-3 생산 코드가 `exec/` 를 **49군데** 쓴다(`kpred` 17 · `stmt_effect` 12 ·
`frame_call` 6 · `frame_window` 2 · `plusargs` 1 …). 그리고 결정적으로 **`exec/process.rs` 안에
`compute_effect` 와 `apply_effect` 가 있다** — tier-3 의 문장 의미 전부다. 그 파일에서 인터프리터
고유인 것은 `run_process` **함수 하나**뿐이다.

⇒ **Phase B 는 "모듈을 지우거나 감싸는 일" 이 아니다.** Phase A 가 끝난 뒤 tier-3 은 사실상 전부를
공유하며, 그것은 결함이 아니라 *"의미의 두 번째 철자를 만들지 마라"* 가 겨눈 바로 그 수렴이다.

**재정의된 Phase B — 제품 표면에서 없앨 것은 코드가 아니라 *선택지*다:**

| # | 무엇 | 크기(실측) |
|---|---|---|
| **B2′** | `Backend::{Interpreter,Bytecode}` 변형과 그 **디스패치**를 `oracle` feature 뒤로 — `scan_arm` 의 match arm · `vm_run_body` · `vm_compiled` · `vm_cache` · `run_process`(그 파일의 나머지 셋은 **남는다**) | 게이트 대상 **≈150–250줄** (삭제 0) |
| **B3′** | CLI 의 `--backend interp\|vm` 철자를 같은 feature 뒤로 | `stage_args.rs`·`frontend.rs` 소수 |
| **B4b** | 그 빌드에서 게이트 거부를 **에러**로(W4030 → E) | 한 자리 |
| **B5** | `--no-default-features` CI 축 | 3-OS |

⚠️ **`#[cfg]` 팬아웃이 진짜 비용이다** — `Backend` 는 public enum 이고 참조가 **73곳**(45+28)이며
대부분이 테스트다. feature 를 켠 개발 빌드에서는 전부 그대로 컴파일되어야 하므로 **테스트는 손댈 필요가
없고**, 손대야 하는 것은 제품 코드의 match arm 들뿐이다. 그것이 이 재정의가 옛 계획보다 싼 이유다.

⚠️ **`exec/process.rs` 는 통째로 감쌀 수 없다.** `enter_body`·`compute_effect`·`apply_effect` 는
tier-3 것이고 `run_process` 만 인터프리터 것이다 ⇒ **함수 단위 `#[cfg]`**, 모듈 단위가 아니다.

#### 5.1-as ✅ B2′+B3′+B5 — **`oracle` feature: 제품 표면에서 없앤 것은 코드가 아니라 선택지다** (2026-08-16)

§5.1-b2 가 착수 전 측정으로 계획을 뒤집은 직후, **재정의된 형태를 그대로 지었다.**
`sim-engine`·`cli` 에 **`oracle` feature(기본 ON)** 를 넣고 `Backend::{Interpreter,Bytecode}` 와 그
**디스패치**를 그 뒤로 보냈다. **삭제한 줄은 0 이다.**

**제품 형태(`--no-default-features`)의 실측 동작:**

| 명령 | 결과 |
|---|---|
| `vita design.sv` | 정상 실행(native) · exit 0 |
| `vita --backend native design.sv` | 정상 실행 · exit 0 |
| `vita --backend vm design.sv` | **`error[VITA-E0001]` · exit 3** |

⭐ **거부 문구가 "모르는 값" 이 아니라 "이 빌드에 없는 실행기" 다.** 두 철자를 match 에서 그냥 빼면
`_ => None` 으로 떨어져 *"unknown value"* 라고 말하는데, 그것은 **다르고 더 나쁜 메시지**다 — 값은
알려진 값이고, 없는 것은 이 **빌드**다. 조용히 받아들이는 것은 더 나쁘다(사용자가 native 를 받고
아무 말도 못 듣는다).

⚠️⚠️ **feature unification 함정을 그대로 밟았고, 빌드는 초록이었다.** `cli` 의
`[dependencies] sim-engine` 에 `default-features = false` 가 없어서 sim-engine 자신의
`default = ["oracle"]` 이 그 평범한 의존을 타고 들어왔다 — `cargo build -p cli --no-default-features`
가 **성공했고 feature 는 아무것도 안 했다**(`cargo tree -e features` 로 확인: `sim-engine feature
"default"` 가 그대로 켜져 있었다). 고친 뒤에야 진짜 팬아웃이 드러났다: **sim-engine 3곳 + cli 7곳.**
계획이 경고한 바로 그것이고, **경고를 읽고도 밟았다** — 그래서 `Cargo.toml` 그 줄에 이유를 적었다.

⭐ **팬아웃이 측정대로 작았다** — sim-engine 은 `scan_arm` 의 디스패치 arm · `init_diag` 의 시작값 ·
그리고 dead-code 가 된 VM 멤버 넷(`vm_run_body`·`vm_compiled`·`vm_cache`·풀 둘). cli 는 파싱·이름
짓기 세 자리. **테스트는 한 줄도 안 고쳤다** — 개발 빌드는 feature 가 켜져 있어 세 백엔드가 전부
그대로 컴파일되고, 그것이 이 재정의가 옛 "삭제" 계획보다 싼 이유다.

**B5 = 별도 CI 축**(`build-no-oracle`). ⚠️ `--workspace` 가 아니라 **`-p cli -p sim-engine`** 이다 —
워크스페이스의 dev-dependencies 가 테스트 타깃을 위해 `sim-engine` 을 기본 feature 로 끌어와서
`oracle` 을 되살리고, 그러면 이 축이 아무것도 시험하지 않는다. 스모크는 두 가지를 본다: 설계가
**돌고**, 없는 실행기 철자가 **거부된다**.

⚠️ 절차: 로컬 검증에서 `target/debug/vita` 가 **두 feature 구성이 공유하는 경로**라 stale 바이너리가
*"`--backend vm` 이 받아들여졌다"* 는 거짓 신호를 냈다(기존 staged-bins 함정과 같은 부류). CI 는 새
체크아웃이라 무관하지만, 로컬에서는 **재빌드 후 재측정**해야 한다.

전 스위트 **5470 green**(기본) · clippy **양쪽 구성 0** · fmt 0.

#### 5.1-at ✅✅ B4b = **Phase B 완료** — 폴백이 없는 빌드에선 거부가 치명이다 (2026-08-16)

⚠️⚠️ **이것은 개선이 아니라 B2′ 가 연 구멍의 수정이다 — 그리고 그 구멍을 찾은 것은 프로브다.**
B2′ 가 폴백 arm 을 feature 뒤로 보내면서 **이 빌드에서 게이트 판정을 소비하는 유일한 자리를
없앴고**, `simulate` 는 그 설계를 **tier-3 로 그냥 돌렸다.** `--no-default-features` 바이너리에
강제 거부를 심어 재니 — **설계가 돌았고 exit 0 이고 진단이 0 이었다.** 게이트가 *"범위 밖"* 이라고
했는데 아무도 안 들었다.

**수정 = 선택 지점에서 치명 + 실행 자체를 건너뛰기.** ⚠️ **래치만으로는 부족했고 그것도 실측이다** —
`fatal_run` 은 `had_fatal`/`finished` 를 세우지만 **건너뛰지는 않으므로** 실행이 계속돼
`NetArena::build` 의 `expect` 에서 **패닉**했다. 그래서 실행기 선택이 `st.finished` 를 먼저 묻는다.

**두 빌드가 같은 사실에 각자 옳은 답을 낸다(실측):**

| 빌드 | 거부된 설계 | 이유 |
|---|---|---|
| 기본(`oracle` ON) | `warning[W4030]` + **VM 폴백** · exit 0 | 폴백은 **느린 답**이지 틀린 답이 아니다 = correct-support |
| 제품(`--no-default-features`) | `fatal[F4004]` · **exit 1** | 폴백 대상이 없으니 선택지가 **loud 아니면 wrong** |

⭐ **새 코드가 필요 없었다** — `fatal_run`(`F-RUN-FATAL`)이 정확히 그 기제이고(graceful fatal ·
`had_fatal`/`finished` 래치) 메시지가 **거부한 행과 왜 폴백이 없는지**를 함께 말한다.

⭐ **그 빌드가 이제 테스트를 갖는다.** `--no-default-features --lib` 이 컴파일되도록
`native::run_tests` 와 `native::frames_tests` 를 **모듈 단위로** `oracle` 뒤로 보냈다 — 둘 다
인터프리터/VM 대비 **차분**이라 제품 형태에는 비교 상대가 없다(사이트 10곳이 전부 그 두 모듈 안이었고,
단언마다 `#[cfg]` 를 다는 것은 썩는다). **147 테스트**가 그 축에서 돈다.

**B5 축에 그것을 실행하는 단계를 더했다**(`cargo test -p sim-engine --no-default-features --lib`).
⚠️ `--lib` 인 이유는 통합 타깃이 dev-dependency 로 `oracle` 을 되살리기 때문이다.

**⇒ Phase B 종료.** B1(기본값 전환) · B4a(교체 경고) · B2′/B3′/B5(feature) · B4b(제품 빌드 치명).
다음은 **Phase C** — `--backend interp` 를 제품 표면에서 빼고 테스트 전용 오라클로. ⭐ 그 절반은
이미 끝났다: `oracle` feature 가 곧 그 경계이고, C 는 그것을 **문서·정책으로 확정**하는 일이다.

전 스위트 **5470 green**(기본) · **147 green**(제품 형태 lib) · clippy **양쪽 0** · fmt 0.

#### 5.1-b1 ⚠️ B4 를 통째로 loud 로 만들면 **사다리 하강**이다 (2026-08-16 측정)

폴백은 **틀린 답이 아니라 느린 답**이다(VM 이 내는 값은 correct-support). 따라서 기본 빌드에서
거부를 `exit≠0` 으로 만드는 것은 **correct-support → loud** = **사다리 하강**이고, 이 저장소가
금지하는 것이다.

⇒ **B4 는 둘로 갈린다.**

* **B4a(경고 · 기본 빌드)** — 값은 그대로 두고 *"native 를 요청했는데 vm 으로 돌았다"* 를 **말한다.**
  하강이 아니고, 실제로 이 프로젝트를 물었던 갭이다(§5.1-o: `run.json` 을 안 봤으면 그대로 배송할
  뻔했다). ⚠️ **오늘은 공허하다**(거부 0) — fail-closed 로 짓고 합성 테스트로 핀해야 한다.
* **B4b(에러 · `--no-default-features`)** — 폴백 대상이 **컴파일되지 않은** 빌드에서는 선택지가
  `loud` 아니면 `wrong` 뿐이므로 하강이 아니다. 이것이 원래 계획이 노린 사다리 상승이고, **B3·B5 가
  선행조건**이다.

⚠️ **"정확히 하나" 는 불가** — 개발 빌드는 in-process 차분이 필요하다(`Backend::` 를 참조하는
테스트 파일 8개). 형태는 **제품=native 하나 · 개발/테스트=둘**.
⚠️ **feature unification** — 워크스페이스 한 곳이라도 `oracle` 을 켜면 전부 켜진다 ⇒ no-oracle
빌드는 **별도 CI 축**이어야 존재한다(B5 가 B3 의 선행조건이 아니라 **쌍**인 이유).

> ## ✅✅ **Phase B 완료 (2026-08-16 · §5.1-aq·-ar·-as·-at)**
>
> **빌드가 둘이다.**
>
> | | 기본(`oracle` ON) | 제품(`--no-default-features`) |
> |---|---|---|
> | 실행기 | `interp` · `vm` · **`native`**(기본) | **`native` 하나** |
> | `--backend vm` | 동작 | **`error[E0001]` · exit 3** |
> | 게이트 거부 | `warning[W4030]` + VM 폴백 · exit 0 | **`fatal[F4004]` · exit 1** |
> | 테스트 | 전 스위트 5,470 | `-p sim-engine --lib` **147** |
> | CI | ubuntu·macos·RHEL9 | **`build-no-oracle`**(빌드+clippy+lib 테스트+스모크) |
>
> ⚠️⚠️ **가장 큰 발견은 계획이 틀렸다는 것이었다**(§5.1-b2) — 옛 B2·B3 는 *"5,430줄 삭제 + `exec/`
> 3,246줄 감싸기"* 였는데 **Phase A 를 지나며 tier-3 이 사실상 전부를 공유하게 됐다**(`backend.rs` 의
> 컴파일 기계장치는 tier-3 의 빠른 경로이고 `exec/process.rs` 안에 `compute_effect`/`apply_effect` 가
> 있다). 그것은 결함이 아니라 *"의미의 두 번째 철자를 만들지 마라"* 가 겨눈 수렴이다.
> ⇒ **제품에서 없앤 것은 코드가 아니라 선택지이고, 삭제한 줄은 0 이다.**
>
> ⚠️ **사다리를 양방향으로 지켰다** — 기본 빌드의 거부는 **경고**(폴백은 느린 답이지 틀린 답이 아니다 =
> correct-support), 제품 빌드의 거부는 **치명**(폴백 대상이 없어 loud 아니면 wrong). 같은 사실에 각자
> 옳은 답이다.
>
> 아키텍처 그림·용어·전체 서사 = **[preview/04 §아키텍처](preview/04-architecture.md)** ·
> **[study/02](study/02-v1-native-coverage.md)**.

### Phase C — V3 + 오라클 재정의

| # | 무엇 | 상태 |
|---|---|---|
| **C1** ✅ | `--backend interp` 를 제품 표면이 아니라 **테스트 도구**로 명시 · **성능 최적화 대상에서 영구 제외** · `Kernel` 제네릭 유지 | **완료 2026-08-17**(§5.1-au) |
| **C2** ✅ | ⭐ **오라클 정책 확정**(§5.1-e) — *"위임 슬라이스는 절대 앵커가 의무"* 를 **빌드의 성질**로 | **완료 2026-08-17** |

#### 5.1-au ✅✅ Phase C — **interp 강등: 코드가 아니라 계약이었다** (2026-08-17)

⭐ **C 의 절반은 Phase B 가 이미 했다** — `oracle` feature 가 곧 그 경계이고, 제품 빌드에는
`Backend::Interpreter` 가 **존재하지 않는다**. 남은 절반은 **그 사실을 계약으로 적는 것**이었고,
그러다 **사용자에게 보이는 거짓말**을 찾았다.

⚠️⚠️ **`--help` 이 한 문단 통째로 거짓이었다.** 그것은:

- *"'vm' (default)"* — B1 이 기본을 옮긴 뒤로 **틀렸다**.
- *"'native' … runs a design when nothing in it is outside today's tier-3 subset —
  **no fork, no `final`, no class, no string net, no $monitor/$strobe**, and of subroutines
  only **FUNCTIONS** whose body stays inside its own frame …"* — **Phase A 가 그 전부를 닫았다.**
- *"measured 1.4x … 0.8x — i.e. SLOWER"* — 낡은 수치.

⭐⭐ **교훈은 그 문단이 잘못 갱신됐다는 것이 아니라 그런 문단을 쓰지 말았어야 한다는 것이다** —
**능력을 열거하는 도움말은 슬라이스마다 썩는다.** 대체한 판은 **각 값의 역할**을 적는다:
*"DEBUG KNOB — 필요 없다. `native` 가 기본이고 모든 설계를 돌린다. 나머지 둘은 같은 의미의 두 번째
구현으로 이분(bisect)하기 위한 것이고, `oracle` feature 없이 빌드하면 아예 없다."* 그리고 실측
수치(interp 1.32 / vm 0.84 / native 0.51 s)만 남겼다.

**C1 의 본체는 `Backend::Interpreter` 의 doc 에 못박은 두 문장이다:**

- ⭐⭐ **"테스트 도구이지 제품 표면이 아니다"** — 그 일은 *읽을 수 있는, 명백히 옳은* 의미의 진술이
  되는 것이다(컴파일 형태도 두 번째 저장소도 없이 `SimIr` 을 직접 걷는다). `vm` 과 `native` 가
  갈렸을 때 중재할 것이 있어야 하기 때문이고, `oracle` feature 가 그 역할을 구조로 만든다.
- ⚠️⚠️ **"성능 최적화 대상에서 영구 제외"** — 관찰이 아니라 **규칙**이다. **레퍼런스를 빠르게 만드는
  것이 곧 레퍼런스가 읽을 수 없게 되는 길**이다: 모든 특수화가 규칙의 **두 번째 철자**이고 그것이 이
  저장소의 결함 클래스다(§4.5.279 — VM 이 인터프리터에서 **조용히 네 갈래로** 갈렸다). ⇒ **프로파일이
  `run_process` 를 지목하면 답은 "그 설계가 여기서 돌면 안 된다" 이지 "여기를 고치자" 가 아니다.**
- ⚠️ **그리고 이것이 `run_process` 를 죽은 코드로 만들지 않는다** — 오라클 빌드에서 VM 은
  `is_codegen_able` 이 거부하는 바디마다 그리로 떨어지고 tier-3 은 프레임 바디를 그리로 위임한다.
  *"제품 표면이 아니다"* 는 **`--backend` 플래그**에 대한 말이다.

**C2 = §5.1-e 를 권고에서 빌드의 성질로.** 제품 빌드에는 **오라클이 없으므로** *"VM 과 바이트
동일은 충분 증거가 아니다"* 는 이제 *"제품에는 그 증거 자체가 없다"* 로 읽힌다 ⇒ **절대 앵커가
유일한 방어선.** ⚠️ 그리고 flip 런 방향이 뒤집혔으므로(§5.1-aq) **`native → vm`** 으로 묻지 않으면
스위트가 조용히 native 전용이 되고 **이 규칙 전체가 공허해진다.**

⭐ **도움말 핀을 약화가 아니라 강화로 갱신했다** — 기존 테스트는 `"byte-identical"` 한 단어만 봤다.
이제 셋을 본다(등가 · **어느 것이 기본인지** · **나머지가 디버그 노브라는 것**) + **Phase A 가 닫은
제약을 도움말이 아직 나열하지 않는지**를 네 문자열로 확인한다 — 같은 부류의 거짓말이 다시 들어오면
그 자리에서 잡힌다.

전 스위트 **5470 green** · 제품 형태 lib **147 green** · clippy 양쪽 0 · fmt 0.

#### 5.1-av ✅⚠️ D1 벤치 확장 — **하네스가 제품 백엔드를 한 번도 안 재고 있었다** (2026-08-17)

⭐⭐ **D1 의 산출은 코드가 아니라 두 개의 숫자 표이고, 둘 다 계획을 바꾼다.**

**형태별 하네스는 이미 있었다**(`perf_baseline.rs` · 8 형태, 답이 형태마다 다르다는 이유로 고른
것들). **그런데 `report()` 가 `interpreter` 와 `bytecode VM` 만 쟀다** — tier-3 의 전 생애 동안, B1 이
`native` 를 기본으로 만든 뒤에도. **이 하네스에서 나온 모든 중단 판정은 tier-2 에 관한 진술이었다.**
형태 커버리지는 있었고 백엔드 커버리지가 없었다.

**실측 (release · best-of-5 · 2026-08-17):**

| 형태 | interp | vm | **native** | native/vm |
|---|---:|---:|---:|---:|
| codegen-heavy(스케줄러 지배) | 34.6 | 24.2 | **22.0** | 0.91× |
| eval-heavy | 386.0 | 104.4 | **60.8** | 0.58× |
| expr-heavy | 1208.8 | 221.2 | **123.5** | 0.56× |
| **struct-heavy** | 417.2 | 96.6 | **112.3** | **1.16× 느림** |
| **wide-heavy(100-bit)** | 466.6 | 211.4 | **360.8** | **1.71× 느림** |
| mem-heavy | 748.9 | 219.4 | **86.1** | 0.39× |
| **wide-struct-heavy(>64-bit)** | 450.8 | 152.3 | **383.8** | **2.52× 느림** |
| real-heavy | 277.7 | 187.5 | **158.9** | 0.85× |

⚠️⚠️ **제품 백엔드가 여덟 중 셋에서 은퇴시킨 백엔드보다 느리다 — 최대 2.52×.** picorv32 하나로는
*"native/vm = 1.63× 빠름"* 이었다. **갈리는 축이 명확하다: 폭이 균일하고 ≤64bit 인 식은 `WProg`
특수화 평가기가 받고, `wide`·`struct` 계열은 안 받는다.** 그것이 D 의 첫 표적이지 2-state 가 아니다.

⚠️⚠️ **그리고 깊이 표는 내가 두 턴 전에 쓴 것을 반증했다.** 그 표도 `Backend::Interpreter` **하나만**
재고 있었다(= S4 중단 판정을 철회시킨 그 표가 **제품 백엔드에서 한 번도 측정된 적이 없다**). 셋으로
넓혀 재니:

| depth | always_comb 체인 (i / v / n) | 인스턴스 체인 = settle (i / v / n) |
|---:|---|---|
| 1 | 3.3 / 2.3 / 2.0 | 4.0 / 3.0 / 2.9 |
| 6 | 8.6 / 4.7 / 3.3 | 12.8 / 9.3 / 7.1 |
| 12 | 15.4 / 8.1 / 5.0 | 25.0 / 17.6 / 12.6 |
| 24 | 31.3 / 17.5 / **8.1** | 55.5 / 41.5 / **23.8** |

**24× 깊이에 세 백엔드 모두 8~14× = 선형이다. 2차 항이 없다.** ⇒ ⚠️ **내가 S4 철회 근거로 인용한
*"7.8 → 814.4 ms = 104×"* 는 `✅ COMB-DEPTH 해결(2026-08-01)` 의 *before* 열이었다** — dirty-settle
이 이미 그것을 고쳤고(after 57.9 ms · 오늘 55.5 로 재현), **Phase D3 는 이미 끝나 있다.**
그 오독을 §5(위)에서 정정했다.

⭐ **그래도 "한 설계면 편향된다" 는 참이었고, D1 이 훨씬 나쁜 형태로 확인했다** — 편향의 방향이
*"이득을 과소평가"* 가 아니라 ***"손해를 못 봄"*** 이었다.

**⇒ Phase D 의 표적이 바뀐다:** D3 는 완료 · **D2 앞에 "wide/struct 축에서 native 가 왜 지는가" 가
온다**(다음 슬라이스의 census 표적). 2-state 좁히기는 여전히 본체이되, **먼저 이미 잃고 있는 것을
되찾는다.**

#### 5.1-aw ✅ D1.5 — **거부의 한정어가 load-bearing 이었고 코드가 그것을 무시했다** · wide 축 회복 (2026-08-17)

D1 이 잰 *"native 가 wide/struct 에서 vm 보다 느리다"* 의 원인은 **census 가 아니라 주석 한 줄**에
있었다. `CompileCtx` 의 `natives` 가 tier-3 에서 `None` 이었고 이유가 이렇게 적혀 있었다:

> *"Tier-3 must not take it … Emitting natives there would swap the faster path for the slower one
> **on every RHS both accept**."*

⭐⭐ **그 한정어가 전부다.** 문장은 **양쪽이 받는 식**에 대해 참이고, **셋째 부류 — `wprog` 가
거부하는 식 — 에 대해 침묵한다.** `wprog` 는 **균일 폭 ≤64bit** 만 받으므로 모든 wide 식이 일반
`eval_ctx` 트리 워크까지 떨어졌고, 그동안 tier-2 는 `native_eval` 로 돌리고 있었다.

⭐ **두 극단을 다 실측했고 doc 은 자기 주장에 대해 옳았다:**

| tier-3 의 natives | expr-heavy | mem-heavy | wide-heavy | wide-struct |
|---|---:|---:|---:|---:|
| `None`(기존) | 157 | 105 | **373** | **399** |
| **무조건 켬** | **478** | **237** | 250 | 179 |
| **`wprog` 가 거부할 때만**(채택) | **153** | **98** | **229** | **165** |

⇒ **스위치가 아니라 분할(partition)이 답이다** — `wprog` 는 자기가 받는 것을 전부 지키고,
`native_eval` 은 나머지를 받는다. 두 평가기가 **경쟁이 아니라 폭으로 공간을 나눈다.**

**같은 세션 A/B(PRE/POST · release · best-of-5) — 정확히 두 형태만 움직였다:**

| 형태 | PRE | POST | |
|---|---:|---:|---|
| **wide-heavy(100-bit)** | 372.6 | **228.9** | **1.63× 빨라짐** · vs vm **1.70 → 1.06** |
| **wide-struct-heavy(>64-bit)** | 398.8 | **165.0** | **2.42× 빨라짐** · vs vm **2.62 → 1.11** |
| 나머지 여섯 | — | — | 노이즈 내 불변 |

⭐ **판정 술어는 추출했지 다시 쓰지 않았다** — `wprog::width_admits` 가 `compile` 의 **첫 줄 그
자체**이고 게이트가 그것을 **묻는다**. 두 번째 사본은 admitted 폭이 움직이는 순간 갈리고, **증상이
느린 경로를 타는 것뿐이라 어떤 테스트도 못 본다.**

⚠️ **`natives_when` 은 `bool` 이 아니라 enum 이다** — *"natives 를 켠다/끈다"* 가 아니라
*"어느 RHS 를 넘긴다"* 가 질문이기 때문이고, tier-2 는 `Always`(잃을 두 번째 평가기가 없다) ·
tier-3 은 `OnlyWhereWprogDeclines`.

⚠️ **no-oracle CI 축이 즉시 값을 냈다** — `NativesWhen::Always` 는 tier-2 전용이라 그 빌드에
생성자가 없고 **`-D warnings` 에서 dead variant 로 잡혔다**. feature 로 갈랐다. **한 실행기만 쓰는
enum arm 은 그 실행기와 함께 사라져야 한다**는 것을 그 축이 강제한다.

⭐ **앵커 둘**(iverilog 핀) — ⓐ **>64bit 가족 한 설계**(워드 경계를 넘는 `+`/`-` · 64 배수가 아닌
시프트 · 하강 part-select 를 포함한 3-파트 concat · 워드 미정렬 replicate · **틀린 상위 워드가 하위
64bit 에 안 보이는** `*`) ⓑ **wide 와 narrow 를 한 설계에**(= 두 극단 중 어느 쪽으로도 만족시킬 수
없는 유일한 모양 · narrow 쪽이 wide 결과를 읽게 지어 분리 최적화를 막았다).

**검증**: 전 스위트 **5472 green** · 제품 형태 lib green · `examples/` 4종 **vm↔native stdout+VCD
바이트 동일** · clippy 양쪽 0.

⚠️ **남은 것: `struct-heavy` 가 여전히 1.25× 느리다**(≤64bit 이라 `wprog` 가 받는다) — **이 슬라이스의
표적이 아니고 별도 census 가 필요하다.** wide 축과 달리 원인이 *"경로가 없다"* 가 아니라 *"있는
경로가 느리다"* 이다.

#### 5.1-ax ✅ D1.6 — **필요조건을 충분조건으로 쓴 대가** · struct 축까지 회복 (2026-08-17)

D1.5 의 분할은 옳았고 **경계가 근사치였다.** *"`wprog` 가 거부할 때만 `native_eval` 로"* 라고 적어 놓고
실제로는 **`wprog::width_admits`**(= `compile` 의 **첫 줄**)를 물었다 — 식을 안 걷고 답할 수 있어
매력적이었지만, **필요조건이지 충분조건이 아니다.** `compile` 은 **노드 종류**로도 거부한다.

⭐ **census 가 그 크기를 쟀다** — 여덟 형태 전반에서 `wprog` 가 **≤64bit 문맥에서 75번 거부**한다
(w=64 에서 65 · w=32 에서 5 · w=16 에서 5). 폭 테스트는 그것들을 전부 통과시켰고, 그러면
`native_eval` 로도 안 가므로 **어느 평가기에도 안 가고 일반 트리 워크로 떨어진다.** 가장 흔한 원인은
**런타임 오프셋 part-select**(`s[idx +: 4]`)다 — `wprog` 는 상수 오프셋만 받는다(§4.5.327).

**수정 = 경계가 `compile` 자신에게 묻는다.** 술어를 **클로저로 넘긴다**(`(rhs, w, signed) -> declines?`)
— `wprog::compile` 은 아레나가 필요하고 아레나는 tier-3 것이라 공유 struct 에 넣을 일이 아니며,
클로저면 **답이 한 철자**(`compile` 자체)로 유지된다.

**같은 세션 A/B — 정확히 한 형태만 움직였다:**

| 형태 | 폭 기반(D1.5) | **실제 판정(D1.6)** |
|---|---:|---:|
| **struct-heavy** | 127.2 (vs vm **1.30×**) | **86.1 (vs vm 0.88×)** |
| 나머지 일곱 | — | 불변 |

⇒ **1.48× 빨라졌고 native 가 VM 을 앞선다.**

**D1 착수 시점 대비 최종 상태(native/vm):**

| 형태 | D1 시작 | **지금** |
|---|---:|---:|
| struct-heavy | 1.16 | **0.88** |
| wide-heavy | 1.71 | **1.08** |
| wide-struct-heavy | **2.52** | **1.13** |
| 나머지 다섯 | 0.39~0.91 | 불변 |

**어떤 형태도 1.13× 를 넘지 않는다**(D1 시작 때 2.52×). ⇒ **D1 이 찾은 회귀 부류가 닫혔다.**

⚠️ **`width_admits` 의 doc 을 고쳤다** — 그 함수의 호출자가 `compile` 하나로 돌아왔고, *"컴파일 시점
호출자가 필요로 하는 것"* 이라던 문장이 거짓이 됐다. 지금은 그 자리에 **왜 그것으로 admission 을
예측하면 안 되는지**가 적혀 있다.

⭐ **앵커**(iverilog 핀) — `s[idx +: 4]` 가 **매 반복 오프셋이 움직이는** 설계에 상수 오프셋 작업과
**나란히** 놓였다(= 분할의 양쪽이 한 바디에서 돈다 · 오프셋을 조용히 얼린 라우팅은 다른 비트 필드에
착지한다).

**검증**: 전 스위트 green · `examples/` 4종 + **picorv32** vm↔native **바이트 동일** · clippy 양쪽 0.

#### 5.1-ay ✅ D2-a — 2-state 레인: 계획은 정적 증명+트랩이었는데 census 가 **동적 검사**를 가리켰다 (2026-08-17)

**ROADMAP 의 D2 스케치는 셋을 요구했다** — 넷마다 *"X 가 못 닿는다"* 를 **정적으로 증명**하고 · 증명
실패 넷은 4-state 로 남기고 · 생성 코드에 **X 진입 트랩**을 심어 correct-or-loud 를 보존한다.
⭐⭐ **census 가 그 셋을 전부 불필요하게 만들었다.**

**census (계측: `WProg::run` 의 leaf 마다 `unk` 를 OR)**

| 대상 | 실행 | 완전 definite | ops | definite 실행의 ops |
|---|---:|---:|---:|---:|
| 벤치 8형태 | — | **8/8 = 100%** | — | 100% |
| **picorv32** | 3,761,534 | **3,389,570 = 90.1%** | 16,050,074 | **91.1%** |

picorv32 의 leaf 중 x/z 를 든 것은 **5.0%** 뿐이다. ⇒ **두 번째 평면은 열 번 중 아홉 번 죽은 일이다.**

**그래서 지은 것은 `run_2s` — 같은 op 열을 한 평면으로 도는 레인**이고, leaf 가 미지를 들고 오는
순간 `None` 을 내면 `run` 이 **정본 루프를 처음부터** 돌린다.

⭐⭐ **정확성 표면이 0이고 그것이 이 설계의 요점이다** — **폴백이 정본 구현 그 자체이지 그것의 근사가
아니다.** 새 의미도, 정적 증명도, 트랩도 없다. 2-state 답은 **모든 leaf 가 definite 였을 때만**
반환되고, definite leaf 위에서 admitted op 은 전부 definite-preserving 이다(그 주장은 기존 소진
배터리가 **이 루프에 대고** 잰다 — `run` 이 디스패치하므로 65,536 상태 스윕의 definite 절반이
그대로 새 레인을 지난다).

⚠️ **재실행이 안전한 이유는 레인에 부작용이 없기 때문이다** — 보고할 수 있는 유일한 팔(`LoadIdx`
범위 밖 = deferred 진단을 센다)이 **보고 전에 bail** 하고, 정본 루프가 정확히 한 번 파일한다.

⚠️⚠️ **그리고 이 모양이 §5.1-ax 의 정확한 대칭이다** — 싼 검사를 비싼 것 앞에 두는 것은 **뒤에 진짜가
있을 때만** 안전하다. D1.5 는 뒤에 아무것도 없는 경계에 근사치를 놓아 조용히 **어느 평가기에도
안 갔다.** 여기서 근사치는 *"아직 미지가 없다"* 이고, 틀리면 **답이 아니라 재실행**을 잃는다.

**같은 세션 A/B(정본 하네스 · best-of-5 · 2회 재현):**

| 형태 | PRE | POST | Δ | vs vm |
|---|---:|---:|---|---|
| **expr-heavy** | 149.8 / 150.1 | **132.0 / 133.5** | **−11.5%** | 0.68 → **0.60** |
| **mem-heavy** | 98.8 / 99.1 | **90.7 / 90.8** | **−8.3%** | 0.46 → **0.43** |
| **struct-heavy** | 84.9 / 85.1 | **80.7 / 81.0** | **−4.9%** | 0.86 → **0.82** |
| 나머지 다섯 | — | — | 노이즈 | 불변 |

⚠️ **첫 판에서 `real-heavy` 가 +5.4% 로 보였고 재측정이 노이즈로 반증했다**(−1.6%) — 이 하네스의
형태별 노이즈는 ±2% 대이고 그 위 하나는 반드시 두 번 재야 한다.

⚠️⚠️ **이득이 5~12% 인 이유를 그대로 적는다 — 그것이 다음 결정의 입력이다.** §4.5.334 프로파일에서
`WProg::run` 은 전체의 **~20%** 다. 그 안을 30% 싸게 만들면 총 6% 이고 **관측값이 정확히 그 범위**다
⇒ **`run` 안쪽 최적화의 천장은 ~20% 이고 D2-a 가 그 중 상당 부분을 가져갔다.** 남은 D2(=**저장소
수준** 2-state 좁히기 — 넷을 아예 한 평면으로 저장해 아레나·쓰기 퍼널의 메모리 트래픽까지 반으로)는
**훨씬 크지만 정확성 거래를 요구한다**(정적 증명 또는 트랩) ⇒ **별도 결정으로 남긴다.**

**뮤테이션 7 · 사망 6 · 생존 1(실측 등가)** — ⭐⭐ **첫 판의 생존 둘 중 하나는 등가가 아니라 눈먼
축이었다**: `LoadIdx` 의 **원소** 미지 검사를 지워도 전 스위트를 통과한다(**인덱스가 definite 인 것이
원소를 definite 로 만들지 않는데**, 배열 원소가 x 를 들고 **런타임 인덱스**로 읽히는 설계가 저장소에
0개였다) → 앵커에 `mem[1] = 8'hx5` 를 `mem[i]` 로 읽는 두 행을 지어 **사살**(`g1=x5 g2=x0` · 값 평면을
믿는 레인은 `05` 를 낸다 · iverilog 핀). 나머지 하나(`Splice` 마스크 제거)는 **실측 등가**다 —
`assert_eq!((p << sh) & !m, 0)` 프로브가 **5,476 테스트에서 한 번도 안 터졌다**(타일링 불변식
`Σ pw == w` 를 컴파일러가 검사한다) ⇒ **마스크는 남긴다**(정본 팔이 하는 것과 같아야 한다 · 쌍둥이가
지키는 가드를 한쪽만 버리면 불변식이 움직이는 날 갈린다).

⚠️ **soundness 렌즈가 내 코드에서 두 번째 철자 하나를 잡았다** — 2-state 의 `Tern` 이 취한 가지를
`t & m` 으로 썼는데 **정본 팔에는 마스크가 없다**(가지 값은 이미 결과 폭으로 마스크돼 있다). 오늘은
no-op 이지만 **불변식이 깨지는 날 이 레인만 그것을 숨긴다** ⇒ 정본과 같은 철자로 고쳤다.

**이빨**: ⭐⭐ 이 슬라이스의 진짜 실패 모드는 **틀린 답이 아니라 레인이 안 도는 것**이다(폴백이
정본이라 `two_state` 를 영구 `false` 로 만들어도 **저장소의 모든 차분이 통과한다**) ⇒ 유닛 테스트가
**공허성부터** 단언한다(256 definite 조합 × 프로그램들에서 `run_2s` 가 **Some 을 내고 `run_4s` 와
같다**는 것을 ≥1,280회 · 미지 leaf 에서 **거부**하는 것 · x 를 든 `Const` 는 **컴파일 시점** 거부).
절대 앵커(iverilog 핀)의 판별자는 **부분 정의 결과**다 — `a | b`(`a=8'hA5, b=8'hxC`)가 `Xd` 이고
미지를 0 으로 읽는 레인은 `ad` 를 낸다(전부 X 인 행은 둘을 못 가른다) · `errors=2` 가 **bail 순서**를
핀한다(보고를 bail 앞에 두면 4).

#### 5.1-az ✅ D4 착수 전 재census — 프로파일이 **cranelift 가 다음이 아니라고** 말했고, 대신 **D1.5 가 뜨겁게 만든 평가기에 빠른 경로가 없었다** (2026-08-17)

ROADMAP 의 D4 행은 착수 조건을 스스로 적어 뒀다 — *"§4.5.334 census 를 **다시** 측정한다."*
D1.5·D1.6·D2-a 가 지나갔으니 그렇게 했다. **산출은 코드가 아니라 프로파일 표이고, 그것이 계획을
다시 정렬했다.**

**프로파일**(release + 심볼 · `/usr/bin/sample` 10 s · 각 형태를 12 s 워크로드로 확대 · self time)

| | mem | expr | struct |
|---|---:|---:|---:|
| `WProg::run` | 40.9% | 46.4% | 22.5% |
| **`native_eval::exec_vm` + `load_scalar`** | 7.7% | 9.5% | **19.3%** |
| `write_chunk_word` | 11.6% | 7.7% | 9.0% |
| `dispatch_body` | 9.9% | 5.8% | 7.6% |
| `Value::{mask_top,resize}` | 3.5% | 4.0% | **10.2%** |
| `drain_range_diags` | 4.5% | 2.5% | 3.2% |

⭐⭐ **`native_eval` 이 struct-heavy 의 1/5 이고 그 옆에 `Value` 마샬링이 10% 다** — §4.5.329~332 가
`wprog` 에서 걷어낸 바로 그 72바이트 표현이, **D1.5/D1.6 이 방금 트래픽을 보낸 레인에** 그대로 있었다.

**원인은 한 줄이고 자기 doc 이 적어 뒀다.** tier-3 의 합성 리더가

```rust
/// The leaf fast path is the ARENA's answer (today: the `NetReader` default
/// `None`, i.e. no fast path).
fn read_scalar_words(...) -> Option<(u64,u64)> { None }
```

⇒ `native_eval` 의 **모든 leaf 로드**가 느린 경로(`read_net` → 72바이트 `Value` → `resize_keep_sign`
→ 두 워드 추출)를 탄다. ⚠️ **그 `None` 이 무해했던 이유가 정확히 한 슬라이스 전에 거짓이 됐다** —
`native_eval` 의 유일한 호출자가 tier-3 인데 D1.5 전까지 tier-3 은 거기에 **아무것도 안 보냈다**
(§4.5.338 의 재발: **기본값은 자기 이유가 언제 거짓이 되는지 모른다**).

**수정 = 추출 + 위임.** 리사이즈 규칙(폭 마스크 → 문맥 부호로 narrowing → **넓힐 때만** 부호확장 →
`w` 마스크)을 `value::scalar_words_resized` 로 **뽑아** 엔진이 위임하고, 아레나가 **같은 함수**로
자기 두 워드를 답한다(합성 리더는 `read_net` 과 **같은 술어**로 frame-local·heap 을 먼저 거른다).

**같은 세션 A/B — 여덟 중 일곱이 개선됐다:**

| 형태 | PRE | POST | Δ | vs vm |
|---|---:|---:|---|---|
| **struct-heavy** | 83.5 | **61.5** | **−26.3%** | 0.88 → **0.60** |
| **expr-heavy** | 135.3 | **119.0** | **−12.0%** | 0.61 → **0.50** |
| **eval-heavy** | 74.1 | **66.0** | **−11.0%** | 0.74 → **0.59** |
| **mem-heavy** | 92.4 | **84.6** | −8.4% | 0.43 → **0.39** |
| **wide-heavy** | 228.9 | **211.8** | −7.5% | 1.09 → **0.99** |
| real-heavy | 179.6 | 168.7 | −6.1% | 0.96 → 0.90 |
| wide-struct-heavy | 168.2 | 158.2 | −5.9% | 1.10 → 1.09 |
| codegen-heavy | 21.2 | 21.5 | 노이즈 | — |

⭐ **wide-heavy 가 처음으로 vm 아래로 내려갔다**(1.09 → 0.99) ⇒ **여덟 중 일곱에서 native 가 VM 을
앞선다**(남은 하나 wide-struct 1.09×).

⚠️⚠️ **그리고 이 슬라이스가 저장소의 거짓 인용 하나를 찾았다** — 엔진의 빠른 경로 doc 이
*"Locked by `leaf_fast_path_matches_read_net`"* 라고 적고 있었는데 **그 테스트는 존재하지 않는다.**
엔진의 빠른 경로는 **쓰인 이래 한 번도 잠긴 적이 없었고**, 이 슬라이스는 거기에 **두 번째 사본**을
더할 참이었다 ⇒ **두 스토어를 함께 잠그는 테스트를 지었다**(각 넷 모양 × 4-state 값 256 × 문맥 폭 8 ×
부호 둘에서 `read_scalar_words` 가 `Some` 을 낼 때마다 `read_net().resize_keep_sign()` 과 비트 동일 ·
거부 셋(배열·real·>64bit)도 단언 · ⭐ **공허성 방지는 "넓히는 signed" 사분면**이다 — 부호확장은
거기서만 일어나므로 그 행이 없으면 확장 블록 전체가 미시험이다).

**뮤테이션 4 · 사망 2 · 생존 2(둘 다 프로브로 **도달 불가** 실측 · fail-closed 유지)** — `arena` 가
부호를 강제하면(**I**) 새 잠금이 즉시 잡고, unk 평면을 버리면(**J**) `casez` 회귀가 잡는다.
⚠️⚠️ **생존 H 는 등가가 아니라 원리적으로 판별 불가였다** — 아레나의 `is_real` 거부를 지워도
스위트가 통과하는데, ⓐ `native_eval::compile` 의 NATIVE-TYPE GUARD 가 **한 층 위에서 real 을 이미
거부**하고(프로브가 제품 경로에서 **0회** 발화) ⓑ **잠금 테스트도 못 잡는다: 워드를 비교하는데
real 은 워드가 같고 다른 것은 `is_real` 스탬프이며 반환형 `(u64,u64)` 에 그것을 실을 자리가 없다.**
⇒ 그 가드는 *"테스트가 요구해서"* 가 아니라 **이미 존재하는 규칙의 스토어 쪽 반쪽**으로 남긴다.
생존 K(합성 리더의 frame/heap 검사)도 프로브 0회 — 같은 이유로 유지.

⇒ **D4(cranelift)는 여전히 마지막이다.** 프로파일이 대는 표적이 아직 **평가기 안**에 있고
(`native_eval` 은 이번에 반쯤 열었을 뿐이다), `dispatch_body` 는 6~10% 다.

#### 5.1-ba ✅ D5 — 융합이 남긴 op: 목적지를 이미 증명해 놓고 라우팅을 다시 걸었다 (2026-08-17)

§5.1-az 의 프로파일이 남긴 다음 표적. **재측정부터** 했더니 그림이 움직여 있었다 —
`Value::{mask_top,resize}` 가 상위에서 **완전히 사라졌고**(직전 슬라이스가 걷어냈다) 대신
**쓰기 퍼널이 struct-heavy 에서 `WProg::run` 보다 커졌다**(31.1% vs 27.2%).

⚠️⚠️ **가설을 두 번 세웠고 두 번 census 가 반증했다.** ⓐ *"`eval_store_word` 가 소스까지 `wprog` 에
요구해서 거부된다"* → 벤치 형태에서 **거부 0**(picorv32 만 6.6%). ⓑ *"목적지가 안 평평해서 일반
퍼널로 간다"* → `compile_body` 는 이미 `(Some(net), false)` 를 `Op::WriteScalar` 로 보낸다.
**세 번째는 추측 대신 계측**했다:

| | `eval_store_word`(평평) | `k_write_scalar`(퍼널) | `k_write_lvalue` |
|---|---:|---:|---:|
| struct-heavy | 900,100 | **300,000** | 305 |
| picorv32 | 691,597 | **102,911** | 120,521 |

⭐⭐ **struct 의 300,000 은 정확히 `acc = acc ^ {12'd0, s[idx +: 4]}`** — **D1.6 의 분할이
`native_eval` 로 보낸 그 문장**이다. 목적지는 `plain_scalar_dest` 가 **컴파일 시점에 평평하다고
증명**했는데, eval 반쪽이 융합되지 않는 순간 저장 반쪽이 `write_routed` 의
heap/assoc/frame/class 라우팅을 **매번 다시 걷는다.**

**수정 = 추출 + 미러링.** `eval_store_word` 의 꼬리(리사이즈→`write_chunk_word`)를
`store_plain_word` 로 뽑아 둘이 공유하고, `k_write_scalar` 가 **정본이 남기는 두 조건만** 본다
(`SimState::write_scalar` 와 같은 둘: `forced` 와 들어온 값의 `is_real` — **둘 다 런타임 성질**이라
컴파일 시점에 답할 수 없다) + `value.width > 64` 는 일반 리사이즈로 떨어뜨린다.

⚠️⚠️ **그리고 이 슬라이스가 성능 회귀를 스스로 만들었다가 대조군으로 잡았다** — 추출만 하고
`#[inline(always)]` 를 안 붙이자 **eval-heavy 가 두 런 연속 +12.5%**(mem +5%) 였다. ⭐ 판별은
**내장 대조군**이 했다: eval/expr/mem/wide 는 `k_write_scalar` 를 **한 번도 안 부르므로** 거기서
움직인 것은 **레이아웃일 수밖에 없다**. 속성을 붙이자 **둘 다 0.4% / 0.2% 로 사라졌다.**

**A/B(정본 하네스 · 세 런)**: **struct-heavy −6.5% / −6.0% / −5.2%**(vs vm 0.63 → **0.58**) ·
나머지 일곱은 이 세션의 노이즈 안(⚠️ load average 6.4 · 대조군이 ±5% 를 보였다).
**이득은 한 형태에 ~6% 이고 그것을 그대로 적는다.**

**뮤테이션 4 · 사망 1 · 생존 3 — 그리고 생존 셋이 전부 값을 냈다.** ⭐⭐ **L(force 검사 제거)이
생존한 것은 그 검사가 애초에 필요 없었다는 뜻이었다** — 읽어 보니 `write_chunk_word` 가 **force
게이트를 자기가 갖고 있고 그 주석이 이유를 적어 뒀다**(*"it must be here rather than only at the
general funnel because `eval_store_word` reaches this method directly"*) ⇒ 쌍둥이 평평 경로는
force 검사를 **한 번도 가진 적이 없고**, 여기에만 넣으면 **두 쌍둥이가 force 를 어디서 답하는지에
대해 어긋난다** = 아무 대가도 없이 산 두 번째 철자 → **지웠다.** **M(is_real 제거)은 사망**
(`cli::real_domain` · 라운드-투-int 팔은 일반 퍼널에만 있다). **N(>64bit 가드 제거)·O(리사이즈
출처 폭)은 등가이고 각각 논거가 있다** — `resize_word` 의 부호확장 팔은 `to_w > from_w` 를 요구하는데
소스가 한 워드 슬롯보다 넓으면 불가능하므로 마스크로 떨어져 **저 워드를 정확히 취한다**(N) ·
`ctx_w = lvalue_width.max(rhs_w)` 이고 평평한 스칼라의 `lvalue_width` 는 곧 슬롯 폭이라
**`value.width >= s.width` 가 항상 참** ⇒ 넓힘이 원리적으로 불가능하고 두 철자가 같은 마스크로
끝난다(O). N 은 **fail-closed 로 남긴다**(그 논거는 테스트가 아니라 읽기다).

#### 5.1-bb ⚪ `drain_range_diags` early-out — **이득 0 을 재고 기록한다** (2026-08-17)

§5.1-az 프로파일이 세 형태에서 **3.1~4.8%** 를 이 함수에 붙였다(그 설계들은 범위 진단을 **하나도**
안 낸다). 원인은 명백해 보였다 — 쌍둥이 둘(`drain_vcd`·`drain_probe`)은 `is_empty()` early-out 을
쓰던 시절부터 갖고 있는데 `drain_range_reports` 만 없어서, **문장마다** `RefCell` 대여 + `Vec`
이동 + drop 을 한다.

⚠️⚠️ **고쳤더니 여덟 형태가 전부 ±2% 안이었다 = 측정 가능한 이득 0.** ⇒ 그 3.1~4.8% 는 드레인이
아니라 **문장마다의 호출 자체와 그것이 하는 세 번의 emptiness 테스트**다. **프로파일 줄은 함수를
이름 부르지 내가 없앨 수 있는 비용을 이름 부르지 않는다.**

⇒ **일관성 수정으로만 배송한다**(쌍둥이 셋이 이제 같은 모양) · 속도 주장 없음 · 그 비용은 **아직
테이블 위에 있고 `dispatch_body` 가 문장마다 세 질문을 그만두게 만드는 슬라이스의 몫**이다.
⭐ early-out 은 **정본 셀을 직접 묻는다**(`take_deferred_range_kinds` 가 읽는 바로 그 `pending_range`)
— 별도 술어는 한 질문에 두 답이고 **틀리는 방향이 진단 소실**이다. 가드를 반전하면 세 테스트가
즉시 죽는다(`s1d4c2c_oob_*`).

#### 5.1-bc ⭐⭐⭐ D6 — 스크래치를 빌린다: **여덟 형태 전부 VM 을 앞선다** (2026-08-17)

⭐⭐ **표적을 정한 것은 콜그래프였다.** `dispatch_body` 의 6~12% 를 미세조정하러 갔는데, 프로파일이
먼저 **`vm_exec` 가 `dispatch_body` 에 통째로 인라인**돼 있음을 보여 줬고(그 줄은 함수가 아니라 **op
디스패치 루프**다), 그 안에 **`_platform_memset` 이 struct-heavy 의 4.5%** 로 앉아 있었다.

**원인은 한 줄이고 자기 주석이 이미 이름을 붙여 뒀다** — `k_eval_native` 가
`NativeScratch::default()` 를 **호출마다** 짓는다:

```rust
// The scratch is per-call here (the engine reuses one behind a RefCell —
// an allocation choice, not a semantic one).
```

`NativeScratch` 는 **고정 배열 둘**(64×16 B + 8×32 B = **1,280 B**)이라 `default()` 가 곧 memset 이고,
struct-heavy 는 native eval 을 **300,000회** 한다 = **384 MB 를 0으로 채운다.**
⚠️ **그 "할당 선택" 이 공짜였던 이유는 D1.5/D1.6 이 거짓으로 만들었다** — 그 전엔 tier-3 이 이
메서드로 아무것도 안 보냈다(§4.5.338 세 번째 재발).

**수정 = `wscratch` 와 같은 모양**(커널의 `RefCell<NativeScratch>` 하나를 **빌린다**).
⭐ **재사용 안전성은 주장이 아니라 실측이다** — 매 호출 직전에 스크래치를 `u64::MAX`/`u128::MAX` 로
**오염시키는 프로브**를 심고 전 스위트를 돌렸다: **5,477 전부 통과** ⇒ 이 호출이 push 하지 않은
슬롯을 읽는 곳이 **없다**(엔진의 사본이 안전한 것과 같은 이유이고, 이제 그것이 측정됐다).

**A/B 두 런 — 여덟 형태가 전부 개선됐다:**

| 형태 | PRE | POST | Δ(런1/런2) | vs vm |
|---|---:|---:|---|---|
| struct-heavy | 55.2 / 57.1 | **50.7 / 50.4** | −8.2% / **−11.7%** | 0.55 → **0.51** |
| wide-struct-heavy | 159.9 / 159.0 | **144.5 / 143.6** | −9.7% / −9.7% | 1.04 → **0.96** |
| eval-heavy | 62.3 / 63.8 | **58.5 / 58.2** | −6.1% / −8.9% | 0.57 → **0.53** |
| wide-heavy | 216.7 / 215.5 | **200.2 / 198.9** | −7.6% / −7.7% | 0.99 → **0.92** |
| expr-heavy | 118.4 / 119.2 | **109.6 / 110.5** | −7.5% / −7.3% | 0.51 → **0.46** |
| real-heavy | 170.8 / 168.6 | **163.1 / 159.5** | −4.5% / −5.4% | 0.92 → **0.88** |
| mem-heavy | 82.9 / 82.6 | **78.6 / 78.6** | −5.2% / −4.8% | 0.38 → **0.36** |
| codegen-heavy | 20.6 / 21.5 | 20.4 / 20.7 | −1.1% / −3.8% | 0.84 → 0.88 |

⭐⭐⭐ **마일스톤: 여덟 형태 전부 `native/vm < 1.00` 이다.** D1 착수 때는 셋에서 지고 있었고
최악이 **2.52×** 였다.

#### 5.1-bd ⚪ 큐의 두 항목을 **재서 닫는다** — `k_write_lvalue` 는 사라졌고 D2-b 는 사다리를 내려간다 (2026-08-17)

**① `k_write_lvalue` — 표적이 아니다(실측).** §5.1-ba 가 picorv32 에서 120,521 회를 세어 큐에 넣었는데,
D5·D6 뒤 재프로파일에서 **`write_routed`·`write_lvalue_inner`·`k_write_lvalue` 가 상위 10에서
전부 사라졌다.** 지금 남은 것:

| | struct | mem |
|---|---:|---:|
| `WProg::run` | 32.2% | 45.4% |
| `native_eval::exec_vm` | 15.1% | 4.3% |
| `write_chunk_word` | 13.8% | 13.5% |
| `dispatch_body`(= 인라인된 op 루프) | 8.9% | 11.3% |
| `k_eval_write_scalar` | 5.5% | 8.0% |
| `run_cached_wprog` | 4.3% | 5.9% |
| `note_change` | 4.0% | 4.5% |

⇒ **남은 비용은 전부 평가기·op 루프·저장 프리미티브**다. 그 셋은 서로 다른 함수가 아니라
**코드 생성기가 통째로 흡수하는 하나**다 ⇒ **미세최적화 큐는 이 형태들에서 고갈됐다.**

**② D2-b(저장소 수준 2-state) — 착수하지 않는다.** 넷을 아예 한 평면으로 저장하면 아레나·쓰기
퍼널의 메모리 트래픽이 반이 되지만, 그러려면 **넷마다 X 가 못 닿음을 정적으로 증명**하거나 **런타임
트랩**을 심어야 한다. ⚠️ **트랩은 사다리 하강이다** — 증명이 틀린 설계에서 오늘 **correct** 인 실행이
**loud** 가 된다(정확성 원칙이 금지하는 방향). 그리고 ⭐ **D2-a 가 이미 그 이득의 무료 절반을
가져갔다**(평가의 90~100% 가 definite 임을 **재서**, 증명 없이 동적으로). 남은 절반은 저장 쪽이고
그 값은 위 표에서 `write_chunk_word` 13.5~13.8% + `note_change` 4~4.5% 이며, **정확성 거래 없이
그것을 가져오는 방법은 오늘 알려진 것이 없다** ⇒ **기록하고 남긴다**(원하면 D4 뒤에 다시 잰다).

⇒ **다음은 D4 뿐이다.**

#### 5.1-be ✅ **D4 = 기계어 코드젠 — 지어서, 배선해서, 재서, 기각한다** (2026-08-17)

**산출 셋: 썩은 feature 를 고쳤고 · tier-3 에 배선했고 · 재니 12~33% 느리다.**

**① `jit` feature 가 컴파일조차 안 되고 있었다.** §4.5.333 이 융합 op 둘
(`Op::EvalWriteScalar`/`EvalNbaScalar`)을 추가한 이래 `cargo build --features jit` 는 깨져 있었다
— 기본 OFF 라 아무도 컴파일한 적이 없다. ⭐ **잡은 것은 이 파일의 `_`-free match 다**(새 op 종류가
조용히 무시되지 않고 **빌드를 세운다**) ⇒ *"쓰이지 않는 코드는 썩는다"* 가 아니라 **"그 부패를
구조가 잡아 준다"** 가 이 저장소의 설계이고, 여기서 그것이 작동했다.

**② 배선은 새 코드가 거의 없었다** — `run_body_jit` 은 처음부터 `&mut dyn Kernel` 을 받으므로
tier-3 의 커널로 그대로 돈다. 없던 것은 **두 번째 호출자**뿐이고, 캐시 조회는 **`Scheduler::jit_body_for`
한 철자**로 뽑아 두 층이 공유한다(⚠️ *"시도했고 거부됐다"* 를 기억하는 `None` 항목이 두 번째 사본이
가장 잘 빠뜨리는 부분이다). ⚠️ **커버리지 계기가 한 실행기만 답하고 있었다** — `VITA_JIT_STATS` 가
엔진 경로에만 있어 **기본 백엔드에서 아무것도 안 찍었다**(= 코드젠 실험이 *"안 도는 것"* 으로 읽힌다).
고친 뒤 실측: picorv32 **34 템플릿 컴파일 · 10 거부 · 462,893 활성화**.

**③ ⚠️⚠️ 그리고 전 스위트를 `VITA_JIT=1` 로 돌리자 **silent-wrong 하나가 나왔다.**
`vm_exec` 는 **문장마다 `k_call_fatal()`** 을 묻는데(그 줄의 주석이 *"REAL and OBSERVABLE"* 이라
적고 사례까지 든다) **컴파일된 바디는 그것을 아예 안 물었다** ⇒ 연속대입에서 시작된 무한 재귀가
평가 안에서 fatal 을 래치했는데 바디가 **그대로 지나가** `Error` 대신 **`Quiescent`** 로 끝났다
(`cont_assign_originated_runaway_terminates`). ⭐ **tier-2 는 이 모듈로 전 스위트를 돌린 적이 없어
아무도 물어본 적이 없었다** — 지는 실험이라도 **진짜 실행기에 배선해서 스위트를 돌려야 하는 이유**다.
고쳤다(`s_call_fatal` + 문장마다 분기 · ⚠️ `k_drain_diags` 는 **일부러 미러링 안 한다** — `vm_exec`
자신의 주석이 그것을 *"measured unobservable backstop"* 이라 기록한다).

**④ 그리고 느리다 — 일관되게(best-of-5 · 위 정확성 수정 포함):**

| | JIT off | JIT on | Δ |
|---|---:|---:|---|
| struct-heavy | 55.1 | 81.2 | **+47.4%** |
| eval-heavy | 62.9 | 87.5 | +39.0% |
| mem-heavy | 81.6 | 107.4 | +31.7% |
| expr-heavy | 116.1 | 148.6 | +28.0% |
| **picorv32** | 514.1 | 586.6 | **+14.1%** |
| wide-heavy | 204.7 | 205.5 | +0.4% |

(수정 전 수치는 +12.5~33.5% 였다 — **문장마다의 정확성 검사가 호출 기반 설계에서 얼마인지**가
그 차이다.)

⭐⭐ **원인은 추론이 아니라 프로파일이다 — JIT 런의 ~38% 가 shim 이다**:
`jit::s_load` **13.7%**(leaf 마다 `k_nets()` = trait object 를 지나는 호출) ·
`jit::mk` **12.4%**(**쓰기마다 72바이트 `Value` 를 다시 짓는다** — §4.5.329~332 가 tier-3 에서 걷어낸
그 마샬링이 **경계에서 부활**한다) · `s_eval_write_scalar` 7.3% · `s_op_select` 4.6%.

⇒ **JIT 은 인라인된 디스패치를 호출들로 바꾼다.** tier-3 의 자기 루프는 `vm_exec` 가 `dispatch_body`
에 통째로 인라인되고 그 안에서 `wprog` 의 2-state 레인과 평평한 저장까지 인라인되는데, 컴파일된
바디는 op 마다 경계를 넘는다.

**⑤ 판정 — 산술이 결론을 낸다.** 완벽한 코드젠이 없앨 수 있는 것은 **op 디스패치뿐이고 그것은
프로파일에서 8.9~11.3%** 다. 지금 경계가 먹는 것은 **~38%** 이고, `mk` 를 통째로 없애도 **~25%** 가
남는다 ⇒ **11% 를 벌려고 25% 를 내는 거래**다. 그 위에 두 번째 대가가 있다: 경계를 없애려면
**표현식 의미를 cranelift IR 로 다시 적어야** 하고 그것이 §4.5.279 가 이름 붙인 결함 부류다
(이 엔진에서 두 번째 구현이 첫 번째와 **네 가지로 조용히 갈린 적이 있다**).

⇒ **코드젠은 기본 OFF 로 남는다.** 다만 이제 **빌드되고 · 배선돼 있고 · 측정돼 있고 · 실행기 둘 다에서
정확하다**(examples 4 · keccak · picorv32 · 전 스위트가 `VITA_JIT=1` 로 초록 · 전부 **바이트 동일**).
**Phase D 종료.**

⭐ **다음에 코드젠을 다시 볼 이유가 생긴다면 조건은 하나다** — leaf 로드와 2-state 산술을 **생성 코드
안에 인라인**할 것(호출 0). §5.1-az 가 아레나에 `read_scalar_words` 를 준 뒤로 leaf 는 **컴파일 시점
인덱스의 워드 두 개**이고, D2-a 뒤로 산술은 **평범한 정수 연산**이다 — 즉 그 전제조건은 이제 존재한다.
없는 것은 그것을 **두 번째 철자 없이** 하는 방법이다.

### Phase D — 기계어 코드젠 · ✅✅ **완료 (2026-08-17)**

⭐⭐⭐ **결과 한 줄: 여덟 벤치 형태 전부 `native/vm < 1.00` 이다** (D1 착수 때는 셋에서 졌고 최악이
**2.52×**). 그리고 **코드젠 자체는 지어서·배선해서·재서 기각했다**(§5.1-be).

| 형태 | D1 착수 native/vm | **종료 시** |
|---|---:|---:|
| wide-struct-heavy | **2.52** | **0.98** |
| wide-heavy | 1.71 | **0.91** |
| struct-heavy | 1.16 | **0.49** |
| eval-heavy | 0.58 | 0.54 |
| expr-heavy | 0.56 | 0.47 |
| mem-heavy | 0.39 | 0.37 |
| real-heavy / codegen-heavy | — | 0.89 / 0.86 |

⚠️ **정직한 한계: picorv32 의 비율은 거의 안 움직였다**(0.61 → 0.60). 벤치 형태는 산술 루프이고
그 설계의 시간은 **다른 데** 있다 — 어디인지는 **아직 안 쟀다**(추측은 근거가 아니다) ⇒ 성능을 다시
본다면 **거기부터 재는 것**이 맞다(아래 §5.2).

⚠️ **D1 을 먼저 하지 않으면 D3/D4 의 판정이 또 편향된다.**

| # | 무엇 | 상태 / 왜 이 자리 |
|---|---|---|
| **D1** ✅ | **벤치를 제품 백엔드로 확장**(§5.1-av) | **완료 2026-08-17.** ⚠️ 형태 커버리지는 이미 있었고 **백엔드 커버리지가 없었다** — `report()` 와 깊이 표가 **native 를 한 번도 안 쟀다** |
| **D1.5** ✅ | **wide 축 회복**(§5.1-aw) | **완료 2026-08-17** — 원인은 `CompileCtx` 주석의 **한정어**였다(*"on every RHS **both accept**"*): `wprog` 가 **거부하는** 셋째 부류에 침묵 ⇒ **분할**(`OnlyWhereWprogDeclines`)로 wide **1.63×·2.42× 빨라짐**(vs vm 1.70→1.06 · 2.62→1.11) · 나머지 여섯 불변 |
| **D1.6** ✅ | **struct 축 회복**(§5.1-ax) | **완료 2026-08-17** — 원인은 *"있는 경로가 느리다"* 가 아니라 **D1.5 의 경계가 근사치**였다는 것(폭은 `compile` 의 첫 줄일 뿐 · **≤64bit 에서 75번 거부**를 통과시켜 어느 평가기에도 안 갔다) ⇒ 경계가 `compile` 에 직접 묻는다 · struct **1.48× 빨라짐**(vs vm 1.30→**0.88**) · **이제 어떤 형태도 1.13× 를 안 넘는다** |
| **D2-a** ✅ | **2-state 레인**(§5.1-ay) — `wprog` 가 한 평면으로 먼저 돌고 미지 leaf 에서 정본 루프로 폴백 | **완료 2026-08-17** — census 가 스케치의 **정적 증명·트랩을 전부 불필요**하게 만들었다(벤치 8/8 이 100% definite · picorv32 90.1%) · **정확성 표면 0**(폴백이 곧 정본) · expr **−11.5%** · mem **−8.3%** · struct **−4.9%** |
| **D2-b** ⛔ | **저장소 수준** 2-state 좁히기 | **착수하지 않는다**(§5.1-bd) — 정적 증명 또는 **X 진입 트랩**이 필요한데 트랩은 **사다리 하강**(오늘 correct 인 실행이 loud 가 된다)이고, D2-a 가 이미 그 이득의 **무료 절반**을 증명 없이 가져갔다 |
| **D5** ✅ | **융합이 남긴 `Op::WriteScalar` 에 평평한 저장 경로**(§5.1-ba) | **완료 2026-08-17** — 목적지를 컴파일 시점에 이미 증명해 놓고 `write_routed` 의 라우팅을 매번 다시 걸고 있었다(struct 의 300,000 = D1.6 이 `native_eval` 로 보낸 그 문장) · struct **−6%** |
| **D6** ✅ | **`k_eval_native` 의 per-call 스크래치를 커널이 빌린다**(§5.1-bc) | **완료 2026-08-17** — 1,280 B 를 **호출마다 memset**(struct 300,000회 = 384 MB) · ⭐⭐ **여덟 형태 전부 개선** 후 **전부 vm 아래로** |
| ⚪ | `drain_range_diags` early-out(§5.1-bb) · `k_write_lvalue`(§5.1-bd) | **이득 0 을 재고 기록** / **재프로파일에서 사라져 표적 아님** |
| ~~**D3**~~ ✅ | **P4 = dirty-driven settle** | ⚠️⚠️ **이미 끝나 있었다(2026-08-01 · `✅ COMB-DEPTH 해결`)** — `ca_deps` + dirty worklist 가 트리에 있고, D1 재측정에서 세 백엔드 모두 **깊이에 선형**(24× 깊이에 8~14×). 내가 §5 에 인용했던 *"104×"* 는 그 수정의 **before 열**이었다(§5.1-av 에서 정정) |
| **D4** ✅⛔ | **코드젠 본체** — cranelift 를 tier-3 에 배선 | **완료 2026-08-17 · 기각**(§5.1-be). ⚠️⚠️ 산출 셋: **feature 가 컴파일조차 안 되고 있었고**(`_`-free match 가 잡았다) · **silent-wrong 하나**(컴파일된 바디가 문장마다의 `k_call_fatal` 을 안 물어 폭주가 `Error` 대신 `Quiescent`) · 그리고 **14~47% 느리다**(런의 **~38% 가 shim** — `s_load` 13.7% · `jit::mk` 12.4% = 72 B `Value` 가 경계에서 부활). **판정 = 산술**: 완벽한 코드젠이 없앨 수 있는 건 op 디스패치 **8.9~11.3%** 인데 경계가 ~38%(mk 를 다 없애도 25%) ⇒ **11% 벌려고 25% 내는 거래** + 경계 제거는 **의미 재작성**(§4.5.279 부류). **기본 OFF 로 남되 이제 빌드되고·배선돼 있고·측정돼 있고·정확하다**(`VITA_JIT=1` 로 전 스위트 green · 바이트 동일) |

---

#### 5.1-f A1 그라운딩 census (2026-08-12) — **계기가 Phase A 의 두 숫자를 정정했다**

전 스위트(5388 tests)를 세 게이트 층을 **독립으로** 물으며 돌렸다 — `simulate()` **6,301 호출**,
네이티브 **4,961 = 78.73%**(기록값과 일치).

⭐⭐ **정정 ①: §5.1-b 의 greedy 표는 이제 stale 하다.** 그 표는 슬라이스 1·2·3 이 닫히기 **전**의
누적치다. 지금 상태에서 **가족 하나만** 닫았을 때의 실측 이득:

| 가족 | 단독 | greedy | 누적 |
|---|---|---|---|
| **A3 서브루틴 프레임** | **+506** | +506 | **86.76%** |
| A2 class+CRV | +182 | +187 | 89.73% |
| **A1 `stmt_effect`** | **+155** | +245 | 93.62% |
| A4 fork | **0** | +97 | 95.16% |
| A5 거부 시스템태스크 | +56 | +81 | 96.45% |
| A6 real | +69 | +74 | 97.62% |
| A7 coverage | +64 | +64 | 98.64% |
| A8 꼬리 | +53 | +64 | 99.65% |
| deferred_assert | +14 | +22 | 100.00% |

⭐⭐ **A3(서브루틴 프레임)가 단독으로도 greedy 로도 1위이고 A1 의 3.3배다.** 오늘 A1 부터 가는 것은
**오너 지시**이고, A3 는 §4.5.338(3b)이 잰 대로 **새 terminator arm + `Kernel` seam** 이 필요한 유일한
가족이라 크기와 난이도가 함께 1위다 — Phase A 의 내부 순서는 A1 이 끝난 뒤 이 표로 재검토한다.
⚠️ **행 단위로 재면 A3 가 사라진다**: `X:CALL STATEMENT` 단독 +71 · `S:task frames` 단독 **+1** —
네 행이 **서로 겹쳐 발화**하므로 **가족 단위로만** 의미가 있다(§5.1-b 의 "행이 아니라 짝" 규칙의 확장형).

⭐⭐ **정정 ②: A1 은 완벽히 분리되는 7개의 sub-slice 다.** `stmt_effect` 단독 차단 **155** 설계의
멤버 집합을 세니 **겹침이 정확히 0**(52+28+26+19+18+8+4 = 155). 각각 독립 배송·독립 측정 가능:

| sub | 멤버 | +설계 | 누적 |
|---|---|---|---|
| **A1-i** | ✅ **queue pop** (`QPopFront`/`QPopBack`) | +18 | 78.98% |
| **A1-ii** | ref-arg 쓰기: seeded `$random`/`$dist_*`(+26) · `$cast` 함수형(+4) · assoc 반복(+28) | +58 | — |
| **A1-iii** | SysTask dest: `$sformat`(+19) · `$readmem*`(+8) | +27 | — |
| **A1-iv** | file 가족: `$fopen`/`$fgets`/`$fgetc`/`$ungetc`/`$feof`/`$fread`/`$fscanf`/`$sscanf` | +52 | — |

⚠️ **A1-i 을 먼저 한 이유는 크기가 아니라 위험이다** — `Scheduler::k_queue_pop` 이 **store-independent**
임을 읽어서 확인했고(입력이 전부 IR·`dyn_heap`·IR 유래 폭), 그래서 **위임 한 줄 + carve-out 한 줄**로
끝난다. 재진술 0 인 슬라이스로 carve-out 기계장치(`stmt_effect_wired`)를 먼저 세우고, 나머지 셋이
그것을 재사용한다.

⚠️⚠️ **`$sformat`/`$readmem*`/`$cast`(태스크형) 셋은 `systask_refusal` 에 없다** — 즉 `k_dispatch_systask`
가 통과시키는데 `readmem(sched, …)`/`cast_task(sched, …)`/Sformat 의 dest 는 **`sched.st.write_lvalue`
= 엔진 스토어**로 쓴다. **지금 그것을 막는 유일한 것이 `stmt_effect` 행이다.** A1-iii 없이 행을 열면
슬라이스 2a 와 같은 silent-wrong 이 된다.

#### 5.1-g ✅ A1-i — queue pop: **커널 코드 한 줄** · 78.73% → **79.03%** (2026-08-12)

지은 것 둘. **`NativeKernel::k_queue_pop` 의 위임 한 줄**과 **`stmt_effect_wired` carve-out**
(§4.5.304 의 `$value$plusargs` 패턴을 리스트로 일반화 — 나머지 세 sub-slice 가 재사용한다).

⭐ **위임이 정당한 이유를 코드에서 읽어서 확인했다** — `Scheduler::k_queue_pop` 이 읽는 것은
`ir.exprs`/`ir.nets`(IR) · `SimState::dyn_heap`(**두 백엔드가 공유하는 한 객체**) ·
`lvalue_width`/`wt`(IR 유래 폭) · `dyn_warn_once_at`(공유 latch)뿐이고 **넷 값을 하나도 안 읽는다.**
목적지 쓰기는 여기가 아니라 `apply_effect` 의 `k_write_lvalue` = 이 커널의 퍼널이다.

**측정**: 재측정 커버리지 **4,961 → 4,982 / 6,304 = 79.03%** · `stmt_effect` 단독 차단
**155 → 137**(−18 = census 예측치와 정확히 일치) · 전 스위트 **5389 green** ·
**flip 런 5386/5389**(실패 3 = 전부 *"기본 백엔드가 vm"* 핀) · **발산 0**.

**뮤테이션 8 중 7 사망 · 1 생존.**

⭐⭐ **B 가 §5.1-e 의 깨끗한 실증이다.** 공유 코드(`Scheduler::k_queue_pop`)에서 front/back 을
뒤집자 **차분(`s1d4c2c_native_run_matches_the_vm_…`)은 killer 목록에 없었고** 앵커 10개가 잡았다.
⚠️ 다만 그 10 중 9는 **기존** iverilog-pinned CLI 테스트다 — 이 함수는 이미 잘 앵커돼 있었고,
내 앵커는 **열 번째**이지 유일한 방어선이 아니었다. 같은 이유로 **G(부호 강제 unsigned)도 기존
`queue_pop_extends_by_element_signedness` 가 이미 잡는다** — F 의 부호 눈멂을 보고 지은 H/I 행은
두 번째 killer 다. **위임 슬라이스의 앵커 의무는 유효하되, 먼저 기존 앵커를 세어라.**

⭐ **H 생존은 등가다(실측).** `popped.resize_keep_sign(lw.max(sw.width), sw.signed)` 에서 `lw` 를
빼도 **concat lvalue(`{a,b}`) · part-select(`p[7:4]`) · 넓은 목적지(`int` ← 4비트 원소)** 셋 다
바이트 동일이다 — `write_lvalue` 가 목적지 폭과 부호를 **다시** 적용하므로 pre-sizing 이 잉여다.
§4.5.338(3a)의 `formal_width` 와 같은 클래스. **기록만 하고 공유 코드는 건드리지 않는다**(별도 정리).

⭐ **프론트엔드가 pop 의 위치를 이미 좁혀 놨다** — `x <= q.pop_back()`(NBA rhs)와 산술 속 pop 은
**loud reject**(`cli::dyn_frontend`). 그래서 게이트의 `BlockingAssign` 전용 스캔이 이 멤버에 대해
**완전**하고, `NonblockingAssign => {}` 팔이 구멍이 아니다. bare `void'(q.pop_front())` 는
`BlockingAssign` 으로 낮춰져 carve-out 을 타고 네이티브로 돈다(iverilog 일치 확인).

⚠️ 절차: 폴링 루프에 `pgrep -f "a1i_battery.sh"` 를 쓰자 **대기 셸들이 자기 명령줄을 매치해**
서로를 살아 있다고 보고 교착했다(메모리에 이미 있는 함정의 재발) → **PID 로 기다려라**.
그리고 `cargo nextest` 를 빌드 중에 죽이면 **잠금이 남아 다음 런이 0% CPU 로 21분 블록**된다.

#### 5.1-h ✅ A1-ii — ref-arg 쓰기 넷: **바디를 `Kernel` 제네릭으로 옮겼다** · 79.03% → **79.94%** (2026-08-12)

`$random(seed)` · `$dist_*(seed,…)` · `ok = $cast(dst,src)` · assoc 반복(`first`/`next`/`last`/`prev`).

⭐⭐ **쓰기는 처음부터 옳았다 — 틀린 것은 읽기였다.** 넷 다 이미 `Kernel::k_write_lvalue` 로
ref-arg 를 썼으므로 목적지는 **부르는 커널의 스토어**로 갔다. 그런데 피연산자 읽기는
`Scheduler::eval` / `SimState::read_net` 라 **엔진의 넷**을 봤다 — 네이티브 런이 한 번도 안 쓰는
그 스토어. ⇒ 수정은 **바디를 `exec::stmt_effect` 로 옮기고 `&mut impl Kernel` 을 받게 한 것**이고,
엔진 경로는 **기계적으로 바이트 동일**하다(그 seam 들이 원래 부르던 바로 그 함수다).

**새 `Kernel` seam 여섯** — `k_eval`(자기결정 평가) · `k_ir` · `k_lvalue_width` · `k_self_width` ·
`k_assoc_iter_cur_key` · `k_assoc_iter_compute`. 전부 읽기 전용이고, 네이티브의 `k_eval` 은
**힙 라우팅**(`SimState::eval_expr_with`)으로 간다 — 슬라이스 2의 `HeapRouted` 가 있는 유일한 프레임.

⭐ **두 번째 철자를 하나 지웠다** — `Scheduler::assoc_iter_step` 삭제. 그것이 프로세스 경로의
반쪽이었고 현재 키를 `self.st.read_net` 로 읽고 있었다. 이제 두 백엔드가 한 바디를 돈다.

**측정**: **4,982 → 5,042 / 6,307 = 79.94%**(예측 +58, 실측 +60) · `stmt_effect` 발화 257 → **120** ·
단독 차단 137 → **80** · 전 스위트 **5390 green** · **flip 런 5387/5390**(실패 3 = 백엔드 이름 핀) ·
**발산 0**.

⭐⭐ **앵커의 절반이 iverilog 핀이다** — `$random(seed)` 는 IEEE 1364-2005 Annex-N LCG 이고
iverilog 가 그 레퍼런스 구현이라 **드로우와 seed 되쓰기 둘 다** 교차검증된다(`$dist_uniform` 도).
⚠️ **`$dist_normal` 은 일부러 앵커에서 뺐다** — 같은 seed 에서 vita 53 / iverilog 54 로 **1 차이**가
나고(seed 는 일치) 이는 `rng::dist_normal` 의 pre-existing 반올림 차이다. **알려진 발산을 앵커에
넣으면 앵커가 앵커이길 그만둔다**(§4.5.302 의 교훈).
⚠️ `$cast` 는 iverilog 13 이 거부 ⇒ hand-IEEE §6.24.2 · assoc 는 iverilog 가 `int aa[int]` 를 못 파싱
⇒ hand-IEEE §7.9.4(**오름차순 방문 · 마지막 뒤 `next` 는 status 0 이고 키를 그대로 둔다**).

⭐ **프론트엔드가 또 범위를 좁혀 놨다** — **string 키 assoc 의 반복은 loud reject**
(*"the iteration key must be an integral VARIABLE"*)라 `k_eval` 의 힙 라우팅이 이 멤버에서는
도달 불가다(그래도 남긴다 — A1-iv 의 `$fgets` dest 가 `string` 이다).

⚠️ **거부 핀 둘이 공허해질 뻔했다** — `effects_outside_the_write_funnel_reject` 의 첫 케이스와
`s1d4a_refused_workers_are_loud_not_silent` 의 둘째가 **`$random(seed)` 를 쓰고 있었다**. 둘 다
**아직 거부되는 멤버**(`$fopen`/`$fgetc`)로 옮겼다 — 안 옮겼으면 행이 살아 있는데 테스트가 사라진다.

#### 5.1-i ✅ A1-iii — SysTask 목적지 쓰기 셋 · 79.94% → **80.40%** (2026-08-12)

`$sformat(dest,…)` · `$readmemb/h(file, mem[, lo, hi])` · `$cast(dst, src)` **태스크형**.

셋 다 `k_dispatch_systask` 를 통과하는데(`systask_refusal` 에 없다) dest 를
**`sched.st.write_lvalue`(엔진 스토어)** 로 썼다 — 막고 있던 유일한 것이 `stmt_effect` 행이었다.

**지은 것 = `TaskWrites` 싱크 하나.** `Direct`(엔진 — 그 사이트들이 원래 하던 바로 그 호출이라
기계적 불변) / `Collect(&mut Vec)`(tier-3 — 목적지 스토어에 닿는 퍼널을 호출자만 들고 있다).
`k_dispatch_systask` 가 dispatch 반환 후 `k_write_lvalue` 로 흘린다. **드레인을 뒤로 미뤄도 관측
불가**: 세 arm 중 방금 쓴 것을 되읽는 것이 없다(`$readmem` 은 토큰을 파싱해 채우고 `$sformat` 은
인자를 먼저 렌더한다).

⭐ **힙 목적지는 이 기계장치가 필요 없다** — `write_lvalue` 가 힙 종류 넷을 **넷 id 로** `dyn_heap`
으로 라우팅하고 그 객체는 공유다. `$s.itoa(v)` 와 `string` 목적지 `$sformat` 은 **원래 맞았고**,
틀린 것은 **flat 목적지**뿐이다(아레나와 `SimState` 가 갈리는 유일한 자리).

⭐⭐ **차분이 내 첫 수정에서 발산을 즉시 잡았다 — 쓰기만 라우팅하고 읽기를 안 했다.**
`cast_task` 가 여전히 `sched.eval_for_lvalue` 를 불러 `sc = 8'd200; $cast(dc, sc);` 가 native 에서
**`dc=x`**(VM 200)였다. §5.1-c 의 교훈(*"행을 열면 그 기능이 부르는 태스크의 인자도 두 번째 store
읽기다"*)의 재발이고, 같은 클래스가 하나 더 있었다 — **`readmem` 의 윈도 인자 `lo`/`hi`**.
⚠️ **첫 프로브가 그것을 놓쳤다**: hex 파일에 `@addr` 지시자가 있으면 **주소를 지시자가 정하므로
윈도가 판별력을 잃는다**. 지시자를 뺀 파일 + 넷 경계(`lo=3, hi=5`)로 앵커를 다시 지었다 —
untreaded 면 `to_u64()` 가 `None` 이 되어 **배열 전체를 조용히 로드**한다.

**측정**: **5,042 → 5,073 / 6,310 = 80.40%**(예측 +27, 실측 +31) · `stmt_effect` 발화 120 → **92** ·
단독 차단 80 → **52** · 전 스위트 **5391 green** · **flip 런 5388/5391** · **발산 0** ·
**뮤테이션 6/6 사망**(B 는 새 `readmem` 앵커만 잡았다).

⚠️ **raw-read 핀이 6 → 4 로 움직였고 그것이 옳다** — 남은 둘은 `writemem` 의 윈도 인자이고
`$writemem*` 은 `systask_refusal` 이 거부한다(메모리 자체를 읽는다).

#### 5.1-j ✅ A1-iv-a — `$sscanf` · 80.40% → **80.72%** (2026-08-12)

⭐ **A1-iv(file 가족 52)를 census 가 다시 쪼갰다** — 멤버 집합을 보면 **`$sscanf` 혼자 20**이고,
그것은 **파일 디스크립터를 안 쓴다**(문자열을 스캔한다). 그래서 file-table 배관 없이 먼저 배송된다.
남은 **fd 가족 32 = A1-iv-b**(`$fopen`/`$fgets`/`$fgetc`/`$ungetc`/`$feof`/`$fread`/`$fscanf`).

**지은 것**: `scan_run`/`scan_next`/`scan_unget`/`scan_write_dst` 를 **`K: Kernel` 제네릭**으로
(A1-iv-b 가 그대로 재사용) + 좁은 seam 둘 **`k_file_read_byte`/`k_file_unget`**.

⚠️ **`k_sched(&mut self) -> &mut Scheduler` 를 먼저 지어 봤고 컴파일이 거부했다** — `Scheduler<'a,'ir>`
는 `'a` 에 대해 **불변(invariant)** 이다. 결과적으로 더 나은 설계가 됐다: 메서드 둘은 공유 바디에
`sched.st.read_net`/`sched.eval`/`sched.st.write_lvalue`(= 엔진의 넷, A1-ii·A1-iii 가 각각 실측한
바로 그 결함)를 **줄 수가 없다**. **구조체를 노출하지 말고 연산을 노출하라.**

⭐ **파일 테이블은 `dyn_heap` 과 같다** — `SimState` 에 있고 두 백엔드가 같은 객체를 본다 ⇒ A1-iv 는
**새 저장소가 아니라 라우팅**이다(슬라이스 2의 결론이 그대로).

**측정**: **5,073 → 5,096 / 6,313 = 80.72%**(예측 +20, 실측 +23) · 전 스위트 **5392 green** ·
**flip 런 5389/5392** · **발산 0** · **뮤테이션 4/4 사망**.
⭐⭐ **넷 중 셋을 새 앵커만 잡았다** — 그리고 그 앵커는 **iverilog 핀**이다(iverilog 는 `$sscanf` 를
구현한다). 판별자는 **B 와 C 의 쌍**: 매치 실패는 **0 이고 목적지를 그대로 두며**, 빈 소스는 **−1**.
소스를 틀린 스토어에서 읽으면 빈 문자열이 되므로 **B 가 −1 로 무너진다** — 한쪽만으로는 못 가른다.

⚠️ `k_read_net` seam 을 지었다가 **지웠다** — A1-iv-b 의 `k_fread` 가 쓸 것인데 이 슬라이스에 호출자가
0이라 dead code 다. **호출자 없는 seam 은 짝이 없는 행과 같다** — 쓰는 슬라이스에서 함께 짓는다.

#### 5.1-k ✅ A1-iv-b — fd 가족 여섯 · 80.72% → **81.13%** (2026-08-12)

`$fopen`·`$fgetc`·`$feof`·`$ungetc`·`$fgets`·`$fscanf`. (`$fread` 만 남는다 = **A1-iv-c**.)

⭐ **파일 테이블은 라우팅이 필요 없다** — `SimState` 에 있고 두 백엔드가 같은 객체를 본다(`dyn_heap`
과 정확히 같다). 그래서 필요한 것은 **좁은 테이블 seam 셋**(`k_file_open`/`k_file_eof`/
`k_file_ungetc`)뿐이고, 나머지는 전부 A1-ii/-iv-a 가 이미 지은 것(`k_eval`·`k_ir`·
`k_write_lvalue`·`k_resolve_lvalue_offsets`·`k_file_read_byte`·`scan_run` 제네릭)이다.

**측정**: **5,096 → 5,124 / 6,316 = 81.13%**(예측 ~+26, 실측 +28) · 전 스위트 **5393 green** ·
**flip 런 5390/5393** · **발산 0**.

⚠️⚠️ **뮤테이션 6 중 2가 생존했고 둘 다 등가가 아니라 내 테스트의 눈먼 축이었다** — 그리고 판별자를
지어 **6/6** 으로 만들었다:
- **C(`$feof` 의 bad-fd 경고 삭제)**: 앵커의 같은 줄에서 `$fgetc` 가 **같은 bad fd** 를 만지고
  `bad_fd_warn` 은 **fd 당 한 번**이라 경고가 그대로 남았다 → **아무도 안 만지는 두 번째 bad fd** 추가.
- **D(`$ungetc` 의 read-capability 검사 삭제)**: 앵커의 모든 pushback 이 **읽기 가능한 fd** 를
  향했다 → **write-only fd 로 pushback**(답은 −1 이고, 요점은 **경고가 없다**는 것 — iverilog 는
  write 스트림을 조용히 거부한다).
⇒ **"경고가 나온다" 는 그 경고의 생산자가 하나일 때만 판별자다.**

⚠️⚠️ **`$fclose` 가 실사용 파일 TB 를 아직 VM 에 묶어 둔다** — 그것은 `systask_refusal`(실행기 층)
이고 fd 를 `int_arg` 로 읽는다(스레드되지 않은 자리). census 의 52 설계는 `$fclose` 를 안 써서 이번
이득에는 영향이 없지만, **진짜 파일 테스트벤치는 파일 호출이 전부 네이티브인데도 안 돈다.**
⇒ **A5(거부 시스템태스크)의 즉시 착수 지점.**

#### 5.1-l ✅ A1-iv-c = **A1 완료** — `$fread` · 81.13% → **81.24%** · `stmt_effect` 행이 사라졌다 (2026-08-12)

`$fread` 는 가족에서 **유일하게 자기 목적지를 읽는다**(원소마다 이전 값과 병합 = `fill_reg_slots`).
seam 셋 추가(`k_read_net`·`k_array_base`·`k_warn_readmem`)로 가족의 **마지막 raw 엔진 쓰기**
(메모리 경로의 `SimState::write_lvalue`, `Kernel` 메서드 안에 있었다)도 사라졌다.

⭐⭐ **A1 종료: `stmt_effect` 가 census 의 design-row 목록에서 완전히 사라졌다.** 전 멤버가 배선됐다
(`every_stmt_effect_family_member_is_wired` 가 15 형태로 핀). ⚠️ **행은 지우지 않았다** — 새 effectful
id 가 추가되면 `sysfunc_is_stmt_effect`/`systask_net_write`(둘 다 `_`-free)가 분류를 강제하고 이 행이
다시 발화한다. 그게 옳다(그 `k_*` 는 아직 없을 테니). 지금 핀하는 것은 **행이 비어 있다**는 사실이다.

**A1 전체**: 78.73% → **81.24%**(+158 설계 · 5,132/6,317). 전 스위트 **5395 green** ·
flip 런 5392/5395 · **발산 0**.

⚠️⚠️ **뮤테이션 하나가 "생존" 으로 보고됐고 그것이 전부 하네스 artifact 였다** — 치환 스크립트가
`assert count==1` 로 죽었는데 러너가 종료코드를 안 봐서 **트리가 안 바뀐 채** 테스트가 돌았고,
손으로 다시 걸었을 때도 **트레이트 스코프 에러로 컴파일이 안 된** stale 바이너리를 실행했다.
제대로 걸자 앵커가 **즉시** 잡는다(부분 읽기가 `4546beef` → `4546xxxx`).
⭐ **그래도 그 가짜 생존이 값을 냈다** — *"왜 두 스토어가 같은 답을 내지?"* 를 묻게 했고, 답은
**`k_read_net` 이 `NetReader::read_net` 의 라우팅을 두 번째로 철자하고 있었다**는 것이었다
(frame-local·heap 은 아레나 것이 아니다). 지금은 정본 라우터를 통과한다.

⚠️ **판별자는 부분 읽기뿐이다** — 원소가 완전히 채워지면 이전 값은 덮여서 안 보인다. 앵커의 P 행이
6바이트를 4바이트 원소 둘에 넣어 둘째 원소의 하위 절반(`beef`)을 남긴다.

#### 5.1-m ✅ A5-a — `$fclose`/`$dumplimit`: **거부 행의 이유가 두 겹으로 stale 했다** · 81.24% → **81.66%** (2026-08-12)

**바꾼 줄 둘.** `$fclose` 의 fd 와 `$dumplimit` 의 size 가 각자 **맨손 `sched.eval`**(= 엔진의 넷)로
읽고 있었다 → `eval_task_arg` 로 스레드하고 `systask_refusal` 에서 행을 지웠다.

⭐⭐ **행이 대던 이유가 두 겹으로 틀렸다** — *"the ARGUMENT is read through `int_arg`, not the
formatter"* 라고 적혀 있었는데 ⓐ **`int_arg` 는 이미 스레드돼 있고**(`eval_task_arg` 를 부른다)
ⓑ **이 둘은 `int_arg` 를 애초에 안 쓴다**(`$timeformat` 만 쓴다). §4.5.338 의 재발:
**거부 행은 자기 이유가 언제 거짓이 되는지 모른다 — 이유가 "…를 통해 읽는다" 형태여도 다시 읽어라.**

⭐ **이것이 A1-iv 를 실제로 쓸 수 있게 만든다** — 파일을 **열고 읽고 닫는** 평범한 테스트벤치가
이제 네이티브로 돈다(그 전엔 파일 호출이 전부 배선됐는데도 `$fclose` 하나가 통째로 VM 으로 보냈다).

**측정**: **5,132 → 5,159 / 6,318 = 81.66%**(+27) · 거부-시스템태스크 실행기 행 **92 → 65** ·
전 스위트 **5396 green** · **flip 런 5393/5396** · **발산 0** · **뮤테이션 2/2 사망**.

⚠️ 앵커에 **알려진 발산 하나를 명시**했다 — 같은 fd 를 **두 번 `$fclose`** 하면 iverilog 는 경고하고
vita 는 안 한다(`bad_fd_warn` 이 fd 당 한 번이고 앞선 `$fgetc` 가 latch 를 이미 썼다). 두 백엔드가
일치하므로 pre-existing 이고, **읽는 사람이 단일 W4022 를 iverilog parity 로 오해하지 않도록 적었다.**

⚠️ 절차: 뮤테이션 둘 다 첫 시도에서 **치환 패턴이 4곳에 맞아** 적용되지 않았고, 그대로였다면
SURVIVED 로 기록됐을 것이다(§5.1-l 의 규칙이 바로 다음 슬라이스에서 재발) → **줄 번호로 지정**.

#### 5.1-n ✅ A3-i — subset 호출: **가족의 크기를 census 가 정정했고, 하네스가 네 번째로 같은 함정을 밟았다** · 81.66% → **84.91%** (2026-08-13)

**측정 먼저.** A3 착수 전 census 를 다시 떴고 **내가 인용해 오던 두 숫자가 둘 다 틀렸다**:

| | 인용해 온 값 | 실측 |
|---|---|---|
| A3 가족 단독 이득 | +506 (→ 86.76%) | **+532 (→ 90.08%)** |
| subset 만의 상한 | 143 (§4.5.338) | **+205** |

⭐ **그리고 가족의 구성이 예상과 달랐다** — 프로세스 바디에 `Terminator::Call` 을 가진 **578** 설계 중
**461 이 태스크를 아예 안 쓴다**(= **output formal 을 가진 함수**). 그쪽은 저장소 행(`is_task`)에
애초에 안 걸리므로 **실행기 arm 하나만** 있으면 된다. 실행기 행 644 회 중 `Terminator::Call` 만이
**577**, `wait fork` 는 **6** 뿐이다.

지은 것:

* **저장소 행을 좁혔다** — `is_task` 전체 거부 → **`is_task ∧ suspendable`**. 판정은
  `sim_ir::compute_suspendable_tasks`, 즉 **엔진이 `simulate` 에서 설치하는 바로 그 집합**을 같은
  입력으로 다시 물은 것(`frames::suspendable_set` 이 유일한 철자)이라 게이트와 `run_process` 가
  같은 태스크를 두고 갈릴 수 없다.
* **실행기 행을 쪼갰다** — `body_is_walkable` 이 `Terminator::Call` 을 `call_ok(bb)` 로 묻는다.
* **`exec::frame_call`** — `run_process` 의 subset arm 을 들어내 `Kernel` 제네릭으로. 새 seam 다섯
  (`k_eval_ctx`·`k_frame_base`·`k_task_call_site`·`k_call_site_runnable`·`k_run_subset_task`).

⭐⭐ **store-dependent 인 것이 거의 없었다 — 프레임 창은 `dyn_heap`·파일 테이블과 같다.**
`frame_stack`/`static_store` 는 `SimState` 에 있고 **두 커널이 같은 객체를 빌린다**. 실제로 갈리는
것은 **① 카피-인**(`split_frame_in_binds` 가 `eval_ctx_top` = 엔진의 넷으로 읽는다 = A1-ii 결함
그대로)과 **② 카피-아웃**(`sched.st.write_lvalue`) 둘뿐이고, 나머지 아홉 호출은
**`SimState::run_subset_task` 하나로 옮겨 양쪽이 위임**한다.

⚠️⚠️ **하네스 갭 네 번째, 그리고 이번 것이 가장 노골적이다** — `build_with_opts` 가
**`task_calls_proc`/`task_calls_func` 를 안 심고 있었다.** `Terminator::Call` 은 `{target, ret_bb}`
만 들고 인자↔formal 매핑이 **통째로 그 사이드카**이므로, 없으면 `run_process` 도 tier-3 도
**`bb = ret_bb` 로 그냥 지나간다** — `r = f(4, o)` 가 `o` 를 안 건드리고 **두 백엔드가 사이좋게 일치**한다.
발견 경로가 요점이다: A3-i 게이트가 호출을 "실행 불가" 로 판정했고 그 이유가 callee 와 무관한
**빈 사이드카**였다(§4.5.337 `assert_ctl` · 2c `queue_slice_stmts` · 3b 원소 둘에 이은 네 번째).
⇒ **사이드카는 선택적 문맥이 아니라 소스의 의미의 일부다** 를 이 파일이 네 번째로 지불했다.

⚠️ **거부 행의 문구를 또 고쳤다**(§4.5.338 클래스) — *"a `wait fork`, or a subroutine CALL
STATEMENT"* 는 walk 이 arm 을 가진 순간 거짓이 됐다. 새 문구는 **터미네이터가 아니라 판정
술어의 말로** 적는다: *"a `wait fork`, a `fork`, or a call statement whose callee suspends"*.

⚠️ **suspendable 검사를 `is_task` 로 스코프한 것은 취향이 아니라 사다리 문제다** — `$display` 가 든
**함수**도 같은 술어로 suspendable 이고, 그런 설계는 **오늘 이미 동작한다**(`Expr::Call` →
`run_frame_call` 경로). 모든 func 에 물었으면 correct → loud **회귀**였다.

**앵커는 절대값이고 iverilog 가 반만 답한다** — iverilog 는 **함수의 output formal 을 거부**한다
(`port twice is not an input port`). A/C/D/E/F/G 는 iverilog 핀이고 **B 는 hand-IEEE**(§13.4.1).
⭐ 판별자를 둘 더 지었다: **F = 두 output formal 을 한 목적지에 앨리어싱**(카피-아웃 **순서**를 볼 수
있는 유일한 행 — 나머지 행은 목적지가 전부 다르므로 순서를 뒤집어도 같은 값이 나온다) ·
**G = 목적지가 곧 입력 actual**(카피-인이 바디 전에 평가된다는 §13.4.1).

**측정**: **5,372 / 6,327 = 84.91%**(자기 테스트 제외 순증 **+208** · 예측 +205) · 전 스위트
**5398 green** · **flip 런 5395/5398**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 9/11 사망**.

⭐⭐ **뮤테이션 넷이 앵커의 구멍 넷을 찾았고, 하나는 "왜 안 죽지" 가 곧 설명이었다** — 카피-인의
**폭**(B)과 **부호**(C), `k_frame_base`(L)를 되돌려도 첫 배터리가 전부 통과했다. 이유는 하나다:
`run_task` 의 **`bind_formal` 이 프레임 진입에서 각 actual 을 formal 의 선언 타입으로 다시
바인딩**하므로, `k_eval_ctx` 에 준 문맥은 **더 좁은 폭에서 평가했다면 값이 이미 파괴됐을 때만**
관측된다. ⇒ 판별자는 **formal 이 actual 보다 넓고 actual 이 자기 self-width 를 넘치는 행**뿐이고
(H·I), 그 둘을 지으니 B·C·L 셋이 동시에 죽는다. 카피-아웃 **순서**(I)도 같은 종류의 구멍이었다 —
목적지가 전부 다르면 순서를 뒤집어도 같은 값이라 **한 목적지에 앨리어싱한 행**(F)이 유일한 판별자다.

⚠️ **생존 둘은 등가가 아니라 도달 불가이고, 그것을 쟀다** — ⓐ `call_site_runnable` 의
`None => false`(사이드카 없는 사이트): 계층 enable 로 지어 봐도 **엔진 시점에는 항목이 있다**
(`missing_sidecar=0` — elaborate 시점에만 없다는 엔진 주석과 일치) · ⓑ `frame_dyn_out_bind` 분기:
**dyn out formal 을 가진 태스크는 전부 suspendable** 이다(`o = new[n]` 은 `SysTask` 이고 `o = i` 는
**handle-copy 마커**라 둘 다 suspend 신호) ⇒ subset arm 에서 도달 불가. **이 코드는 옮겨온 것이지
이 슬라이스가 만든 것이 아니므로**, 죽지 않는 것을 kill 로 위장하는 대신 사실로 기록한다.

⭐ **곁가지로 두 번째 철자 하나를 지웠다** — 지어 놓고 보니 `Scheduler::split_frame_in_binds` 와
`exec::frame_call::split_in_binds` 가 §13.4.3 사이징 규칙을 **두 번** 적고 있었다(clippy 가
`type_complexity` 로 먼저 걸렸고, 고치려다 발견). 엔진 쪽을 **위임 한 줄**로 만들어 철자를 하나로.

#### 5.1-o ✅ A3-ii-a — 구동 프레임: **"라우팅" 이 세 군데 있었고 그 중 하나만 프레임을 알았다** · 84.91% → **88.55%** (2026-08-13)

**census 가 A3-ii 를 다시 쪼갰다.** "suspendable" 은 **엔진이 고르는 실행기의 이름**이지 바디가 park
한다는 주장이 아니다 — `stmt_signal` 은 `$display`·NBA·프레임 밖 쓰기를 신호로 세는데, 그건 동기
`&self` 실행기가 그걸 **못 하기** 때문이지 바디가 멈추기 때문이 아니다. 실측: 프로세스 바디의 callee
가 suspendable 인 **357 설계 중 250 이 `Delay`/`Wait`/`Fork` 를 아예 안 가진다.**

⇒ 그 절반은 **활성화가 `run_body` 호출 하나 안에 완전히 중첩**된다. 프레임 스택과 CFG 루프는 필요하지만
**park/resume·윈도 stash·스케줄러 상태는 전혀 필요 없다.** `OpenFrame` 은 그래서 `Vec` **지역변수**이고,
`FrameRec` 이 가진 `window`/`dyn_parked`/`forked`/`is_arm` 이 **하나도 없다**(전부 이 게이트가 거부하는
중단을 위한 필드다).

⭐⭐ **이 슬라이스의 산출은 "읽기 라우팅이 세 군데 있고 하나만 프레임을 알았다" 는 발견이다.** 프레임
창은 `SimState` 에 있고 두 백엔드가 같은 객체를 빌린다(= `dyn_heap`·파일 테이블과 같다) — 그런데
tier-3 이 프레임 바디를 **실행하기 시작하자** 세 경로가 차례로 틀렸다:

| 경로 | 증상 | 왜 그때까지 무해했나 |
|---|---|---|
| `write_routed` 에 **프레임 레인이 없다** | 바디의 `s = x + y` 가 아레나의 **죽은 슬롯**에 | 읽기 쪽은 S3a 이래 라우팅했다 — **쓰기 쪽에만 짝이 없었다** |
| `wprog::compile` 의 `Signal` | formal 이 전부 `x` | 컴파일 시점에 **슬롯으로 해석**한다(프레임에 눈멀었다) |
| `HeapRouted` | **프레임 안의 `$display`** 만 `x` | 힙만 라우팅했다 — 포매터가 닿는 경로가 정확히 이것 |

셋 다 **`frames_admitted` 의 module-body 행**이 막고 있었을 뿐이다(모듈 바디는 프레임 넷을 이름부를 수
없다). 행이 열리는 순간 셋 다 조용한 오답이 된다. ⚠️ 그리고 셋을 **하나씩** 고쳤는데 매번 출력의 **일부만**
맞아서, 어느 단계에서 멈췄어도 "동작한다" 로 보였을 것이다.

⚠️⚠️ **첫 end-to-end 확인이 공허했다** — native 로 돌렸다고 생각한 설계가 `buildable:false` 로 **VM 에
떨어져** 있었고, 그래서 iverilog 와 완벽히 일치했다. `run.json` 을 안 봤으면 그대로 배송됐다.
⇒ **"두 백엔드가 일치한다" 는 native 가 실제로 돌았을 때만 의미가 있다.**

⭐ **전제조건이 두 실행기마다 다르다.** 위임되는 바디(`run_frame_call`/`run_task_call`)는 엔진의 flat
store 를 읽으므로 **자기 창 밖 넷을 이름부르면 안 된다**(S3a 의 논거). **구동되는** 바디는
`compute_effect`/`apply_effect` 를 타므로 모든 읽기가 `k_read_net`·모든 쓰기가 `k_write_lvalue` 다 —
모듈 넷을 이름부르는 것이 **정상**이고, 위임 쪽 전제조건을 그대로 적용하면 이 슬라이스가 겨냥한 설계가
전부 거부된다(실측). 오직 **태스크만** 구동될 수 있다(반환값이 없으니 `Expr::Call` 로 못 닿는다).

⚠️ **거부 행 문구가 또 움직였고 이번엔 "도달 불가" 를 쟀다** — 호출문의 거부는 이제 **callee 가 park 할
때뿐**인데, park 하는 callee 는 **항상 태스크**다(elaborate 가 함수 안의 타이밍 제어를 E3009 로 거부하고
— iverilog 도 같다 — 함수는 태스크를 enable 할 수 없다). 그리고 park 하는 태스크는 **저장소 행**이 한
층 먼저 잡는다 ⇒ **실행기 행의 그 절 은 오늘 도달 불가**다. 행은 남기고(두 층은 독립으로 질의된다)
`a_parking_callee_is_refused_by_the_storage_layer` 로 따로 핀했다.

⚠️ **`has_hier_call` 이 LIVE 가 됐다 — 이 파일이 두 번 "dead" 라고 적은 행이다.** S3a 는 `is_task` 가
먼저 잡는다고, A3-i 은 "두 플래그 다 suspendable 을 함의한다" 고 적었는데, A3-ii-a 가 non-parking
suspendable 을 받으면서 그 논거가 만료됐다. 실측 **19 설계**가 지금 이 행에만 걸린다.

⭐ **rustc 가 결함 하나를 잡았다** — `Return` arm 의 `else` 를 빠뜨려서 프레임이 pop 된 뒤 그대로
`k_rearm` + `Step::Done` 으로 흘러 **첫 태스크 반환에서 프로세스가 끝났다**. 경고는
*"value assigned to `bb` is never read"* 였다 — **죽은 저장이 곧 버그였다.**

**측정**: **5,605 / 6,330 = 88.55%**(+233 · 예측 +223) · 전 스위트 **5400 green** · **flip 런
5397/5400**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 9/11 사망**.

⭐ **죽은 아홉 중 여덟을 새 절대 앵커 하나가 잡았다** — 위의 세 라우팅 결함이 각각 출력의 **다른 조각**만
망가뜨리기 때문에, 프레임 안의 `$display`·모듈 넷 쓰기·중첩 프레임·재귀·프레임 안 제어흐름·배열 원소
목적지를 **한 설계에 모아** 놓은 것이 그대로 판별자가 된다.

⚠️ **생존 둘은 등가가 아니라 도달 불가이고, 둘 다 쟀다** — ⓐ `frame_suspends` 의 **전이 arm**:
`frames_admitted` 가 **모든 func 에** 같은 질문을 하므로, 중첩 callee 만 park 하는 설계는 **그 callee
자신의 행**이 거부한다(호출자의 답은 아무것도 결정하지 않는다) · ⓑ **frame-local NBA 행**: elaborate 가
그 형태를 **E3009 로 거부**한다(iverilog 도 IEEE §10.4.2 를 인용해 거부). 둘 다 fail-closed 방향이라
남기되 **"덮였다" 가 아니라 "도달 불가" 로 기록**한다.

#### 5.1-p ✅ A2-i — plain OOP: **한 단어짜리 거부 행이 121 대 39 를 가리고 있었다** · 88.55% → **90.59%** (2026-08-13)

⭐⭐ **착수 전 census 가 이 파일이 적어 둔 다음 슬라이스를 취소했다.** §5.1-o 는 *"다음 = A3-ii-b
실제로 park 하는 프레임(+81)"* 로 끝난다. 재측정:

| 후보 | 단독 이득 | 실제 |
|---|---|---|
| A3-ii-b = `a task frame that SUSPENDS` **행 하나** | **+1** | 그 행이 발화하는 81 설계 중 **80 이 실행기 행(`call statement whose callee suspends`)에도 걸린다** |
| PARK(S) + call-stmt(X) = **진짜 A3-ii-b** | **+37** | 게다가 그 80 중 **40 은 `fork`(D) 도** 걸린다 ⇒ A4 하류 |
| **A2 = `class`(D)** | **+160** | 4.3배 · 오너 고정 Phase A 순서에서도 다음 |
| `handle_copy`(D) | +81 | |
| `coverage`(D) | +64 | |

⇒ **행 하나의 이득은 그 행이 발화하는 설계 수가 아니다.** A3-ii-b 는 park/resume·윈도 stash·dyn
park·per-activity 콜스택을 전부 요구하면서 +37 이고, A2 는 라우팅만으로 +160 이다. **A3 는 사실상
끝났다**(가족 잔여 +62).

⭐⭐ **그리고 `class` 행 자체가 121 대 39 를 가리고 있었다.** 열두 사이드카를 하나로 묶은 *"OOP 가
얼마나 있나"* 행이라, `class C; int f; endclass` 를 선언한 설계와 제약을 푸는 설계가 **같은 단어로**
거부됐다. 실측: 그 행이 막던 160 중 **121 이 `randomize()` 를 실행하지 않고 virtual 호출 사이트도
없다**. ⚠️ **사이드카로는 못 가른다** — 160 **전부**가 `class_rand`·`class_vtable` 을 갖는다(둘 다
클래스 단위라 `rand` 필드나 메서드를 **선언만** 해도 생긴다) ⇒ **테이블이 아니라 사이트로 잘라야
한다**(`Stmt::SysTask{ClassRandomize}` 개수).

⭐⭐ **plain OOP 는 저장소가 아니라 라우팅이다** — V1 슬라이스 2(heap)와 같은 발견이다. 클래스 핸들
넷은 **평범한 `Logic` 슬롯**이고(그 값 = 객체 id), 객체의 **필드**는 `SimState::class_heap` 에 있으며
두 커널이 같은 객체를 빌린다. 지은 것:

- **아레나 비트맵 `class`** — `heap`/`frame` 의 세 번째 쌍둥이. ⚠️ **다른 점이 하나 있고 그것이
  핵심이다**: 앞의 둘은 슬롯이 통째로 죽었지만 클래스 핸들의 슬롯은 **반만 죽었다**(핸들 id 는 여기
  있고 필드만 힙에 있다) ⇒ 모든 소비자가 `class[net] ∧ word.is_some()` 로 물어야 한다. 비트맵만 보고
  라우팅하면 **맨 핸들 읽기가 아무것도 없는 힙으로** 간다.
- **`SimState::handle_id_with(nets, net)`** — 이것이 읽기 쪽 슬라이스 전부다. 옛 `read_handle_id` 는
  `self.read_net` = **엔진의 flat store** 를 읽었고, 네이티브 런은 그 store 를 t0 에 남긴다 ⇒ 모든
  `obj.f` 가 핸들 **0 = null** 을 역참조한다. **loud 가 아니다** — null 은 정의된 의미이고 경고도
  그럴듯하다. **A1-ii 그대로**(*쓰기는 이미 옳았고 틀린 것은 읽기였다*)이고 수정도 A1-ii 그대로다:
  연산은 한 철자로 여기 남고 **store 가 파라미터로 온다**.
- 라우팅 네 자리 — 읽기 퍼널(`NativeKernel::read_net`, **frame/heap 보다 먼저**: 메서드의 `this` 는
  핸들이면서 frame-local 이다) · 쓰기 퍼널(`write_routed`) · **`wprog` 거절** · **`HeapRouted`**
  (`k_dispatch_typetask` 가 포맷터에 **맨 아레나**를 넘기므로 `$display("%0d", p.x)` 는 여기로 온다).
- **`k_class_alloc` 위임 한 줄**(A1-i 의 `k_queue_pop` 과 같은 이유 — 넷을 하나도 안 읽는다).

⭐⭐ **계획에 있던 두 번째 거부 행(`class_virtual`)을 실측이 죽였다** — `resolve_virtual_call` 은
`args[0]`(호출자가 자기 store 에서 이미 평가한 수신자 핸들 **값**)과 공유 테이블 둘만 읽고 **넷을 하나도
안 읽으며**, 두 composite 가 모두 `st` 로 forward 한다. 3단 상속 + 각 단계 override + 상속 메서드 +
base 핸들 재지정 설계가 **네이티브로 돌고 3-way 일치**한다 ⇒ 행을 지웠다(**과잉거부는 사다리 하강**).

⭐ **오라클이 예상 밖이었고 반쪽이다** — **iverilog 13 은 SV 클래스를 지원한다**(N7 이래 이 저장소가
무오라클로 취급해 온 영역). 그래서 앵커가 hand-IEEE 가 아니라 **`vvp` 핀**이다. 단 두 군데가 예외이고
둘 다 쟀다: ⓐ **null 핸들 역참조는 `ivl` 이 컴파일 중 SEGFAULT** ⇒ 별도 테스트로 hand-IEEE ·
ⓑ **virtual dispatch 를 iverilog 가 틀린다**(`B h = d; h.who()` 에서 `B::who` 호출 · IEEE §8.20 은
동적 타입의 override 요구) — vita 세 백엔드가 LRM 과 일치하므로 **이 항목은 vita 가 오라클보다
앞선다**. **알려진 발산을 앵커에 넣으면 앵커가 아니게 되므로**(§4.5.302) 셋을 각각 다른 테스트에 핀했다.

⚠️⚠️ **하네스 갭 다섯 번째** — `build_with_opts` 가 클래스 사이드카를 안 심고 있었다. `class_field_widths`
만 §4.5.309 가 넣어 둬서 **덮인 것처럼 보였고**, 정작 `class_handle_nets`(= `SimState::class_is_handle`
**와** 아레나 비트맵을 **둘 다** 채우는 테이블)가 없었다 ⇒ 그것 없이는 `o.f = 7` 이 필드 접근이
아니라 핸들 슬롯 자체에 대한 쓰기가 되어 **두 백엔드가 똑같이 틀린 일을 하고 일치**한다. ⭐ **이번엔
실패가 아니라 그 파일의 앞선 네 개 주석을 읽어서 찾았다** — 그래서 다섯 번째 항목이 아니라 예방이다.

**측정**: **5,744 / 6,341 = 90.59%**(예측 +121 plain + 7 virtual = +128 · 실측 **+128**) · 전 스위트
**5406 green** · **flip 런 5403/5406**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 10/11 사망 ·
생존 1 은 등가(실측)**.

⚠️ **첫 배터리는 9/11 이었고 생존 둘 다 공유 코드였다**(§5.1-e 재발) — `class_field_write_with` 의
**2-state 강제**와 **필드 폭·부호 리사이즈**. 앵커에 줄 둘을 지어 다시 물었다: `4'bx1z0` 을
`bit[3:0]` 과 `logic[3:0]` 에 **나란히** 쓰고(§6.11.3 강제만 차이가 된다), 64비트 값을 `int`/`byte`
필드에 쓴다(절단·부호가 보인다).

- **2-state 강제는 눈먼 축이었다** — 앵커가 필드에 X 를 한 번도 안 넣고 있었다 → 새 줄이 즉시 죽인다.
- ⭐⭐ **리사이즈는 진짜 등가이고, 그것도 실측이다.** 계측해 보니 그 줄은 **실제로 64비트 값을 32비트
  필드에 받고 있는데**(예상대로) 지워도 출력이 안 바뀐다 — **읽기 쪽이 다시 좁히기** 때문이다:
  `class_field_read` 는 저장된 `Value` 를 그대로 돌려주고 `eval_ctx` 가 문맥 폭으로 사이징하는데,
  그 문맥 폭을 `patch_class_fields` 가 이미 **필드의 폭·부호**로 바꿔 놓았다. ⇒ **이 저장소가 같은
  모양을 네 번째로 쟀다**(프레임 formal 의 `bind_formal` §5.1-n · 목적지 폭의 `write_lvalue`
  A1-i `H` · 호출 actual 의 `formal_width` §4.5.338). 줄은 fail-closed 방향이라 남기고 **"덮였다" 가
  아니라 "등가" 로 기록**한다.

#### 5.1-q ✅ A8-a — `handle_copy`: **거부 행이 순전히 보수적이었다 · 커널 코드 0줄** · 90.59% → **91.86%** (2026-08-13)

⭐ **§5.1-p 가 기록한 규칙을 바로 적용해서 표적이 정해졌다** — 슬라이스 착수 전 census 재측정에서
`handle_copy` 가 **+81 로 1위**로 올라와 있었다(A2-i 이전 census 에서도 +81 이었지만 `class` +160 에
가려 있었다).

**지은 것은 없다.** 지운 것은 `design_eligibility` 의 `handle_copy` 행 하나다. 전 구현이
`builtins::dispatch` 안에 있고 tier-3 은 S1d-4b 이래 그 함수를 통과한다:

- `dst = src`(IEEE §7.10)는 **no-op `Display` + StmtId → `(dst_net, src_net)` 마커**로 낮춰진다.
- 그 arm 은 **`dyn_heap[src]` 를 깊은 복사해 `dyn_heap[dst]` 에 넣고** `enforce_queue_bound(dst)` 를
  부른다. `dyn_heap`(V1 슬라이스 2)·`handle_copy_stmts`·`queue_bounds`·warn 래치가 **전부 `SimState`
  이고 넷 id 로 키잉**되며, **두 넷 id 는 평가가 아니라 사이드카에서 온다.**
- ⇒ **어느 경로에서도 넷 값을 하나도 안 읽는다.** 라우팅할 것이 애초에 없었다.

V1 슬라이스 1(SVA)·A1-i(`k_queue_pop`)와 같은 모양이다 — **거부 행이 기능의 이름을 댔지 이 백엔드에
없는 기계장치를 댄 것이 아니었다.**

⚠️⚠️ **하네스 갭 여섯 번째, 그리고 다시 예방이었다** — `build_with_opts` 에 `handle_copy_stmts` 가
없었다. 그것 없이는 `d2 = d1` 이 **아무것도 출력하지 않고 아무것도 복사하지 않는** no-op `Display` 이고,
**두 백엔드가 아무도 수행하지 않은 깊은 복사에 대해 일치한다.** §5.1-p 가 다섯 번째를 같은 방식으로
잡았고 이번에도 코드를 쓰기 전에 표를 세어서 찾았다.

⭐ **오라클이 또 반쪽이다** — iverilog 13 은 dyn array·queue·`string[]` 사본을 전부 답하지만
**assoc 배열은 파싱조차 못 한다**(`int a1[int]` → *"Type names are not valid expressions here"*). ⇒
앵커를 **둘로 쪼갰다**: iverilog 핀(dyn/queue/string) + hand-IEEE(assoc·bounded queue §7.10.2 절단).
**오라클이 답할 수 있는 선에서 자르는 것**이 앵커를 약화시켜 맞추는 것보다 낫다(§4.5.302).

**측정**: **5,827 / 6,343 = 91.87%**(+81 · 예측 +81 · 모집단이 새 테스트 둘만큼 늘었다) · 전 스위트 **5409 green** · **flip 런
5406/5409**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 6/6 사망**.

⚠️ 절차: 첫 배터리에서 케이스 A 가 **BUILD-FAIL**(치환이 타입 오류를 냈다)이었고, §5.1-l 규칙대로
**SURVIVED 로 세지 않고** 치환을 고쳐(`.filter(|_| false)`) 다시 걸었다.

#### 5.1-r ✅ A7 — functional coverage: **거부는 보수적이었고, 진짜 결함은 실행이 아니라 보고에 있었다** · 91.87% → **92.88%** (2026-08-13)

⭐ **커버그룹은 런타임 기계장치가 아니다** — V1 슬라이스 1 이 SVA 에 대해 한 발견과 같다. elaborate 가
`cg.sample()` 을 **비트맵 넷에 대한 평범한 비트 세트 대입**(`1 << (v & 63)`, 명시 bin 은 그 등가물)으로,
`get_coverage()` 를 **그 넷 위의 평범한 산술**로 desugar 한다 ⇒ 엔진에 도착하는 것은 평범한 IR 이고
tier-3 워크는 **바디를 실행할 수 있게 된 이래 커버그룹을 옳게 실행해 왔다.**

⚠️⚠️ **못 한 것은 실행이 아니라 보고였고, 그것이 이 슬라이스의 본체다.** 런 종료 요약이
`st.nets[it.bitmap_net].cur` — **엔진의 flat store** — 를 읽는데 네이티브 런은 그것을 안 쓴다. 행만
지우고 그 수정 없이 배송했으면 **exit 0 에 `coverage_pct: 0.00`** 을 발행했을 것이다: 크래시가 아니라
**G2 산출물(`coverage.json`) 안의 silent-wrong** 이고, **0.0 은 합법적인 값이라 아무도 못 알아본다.**

수정 = `simulate` 가 **아레나가 drop 되기 전에** 각 아이템의 최종 비트맵을 거둔다(`cover_bits`). 셋:

- **composite 리더로 읽는다**(`nk.read_net`, `arena.read_net` 이 아니라) — 오늘 비트맵은 모듈 스코프
  packed `logic` 이라 둘이 같지만, 힙/프레임 넷이 되면 소유자가 답해야 한다.
- **엔진 경로는 `None`** 이고 그래서 바이트 동일하다 — 요약이 늘 하던 그 읽기로 그대로 떨어진다.
- **인스턴스를 가로지르는 flat 인덱스 하나** — 인스턴스마다 리셋하면 첫 그룹의 bin 이 둘째 이름으로
  보고되고, 두 coverpoint 의 bin 수가 같으면 **그럴듯한 숫자가 나온다**(차분 케이스로 고정).

⚠️⚠️ **하네스 갭 일곱 번째, 그리고 실패 모드가 이 목록의 존재 이유다** — `build_with_opts` 에
`coverage_manifest` 가 없었다. 없어도 설계는 **돌고 비트맵 비트도 세팅된다** — 다만
`SimResult.coverage == None` 이 될 뿐이라, 요약을 단언하는 테스트가 **`None` 과 `None` 을 비교하며
통과**하고 정작 검사하려던 store 라우팅은 한 번도 안 돈다.

⭐ 앵커는 hand-IEEE 다(`iverilog 13` 은 covergroup 을 통째로 거부 — `cli/tests/coverage_n5.rs` 헤더).
숫자는 전부 유도했다: `x = 0..5` 는 `lo`{0:3}·`mid`{4:7} 만 맞히고 `hi` 는 못 맞힌다 ⇒ 2/3 ·
`y = i[1:0]` 은 0,1,2,3,0,1 ⇒ 4/4 · cross 는 3×4=12 bin 에 서로 다른 쌍 6개 ⇒ 6/12 ·
평균 (200/3 + 100 + 50)/3 = **72.222222**. ⭐ **앵커의 마지막 줄이 공허성 방지다** —
`coverage_pct > 0.0`, 왜냐하면 **0.0 이 바로 미수확 네이티브 런이 발행하는 값**이기 때문이다.

**측정**: **5,898 / 6,350 = 92.88%**(+64 · 예측 +64) · 전 스위트 **5412 green** · **flip 런
5409/5412**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 5/5 사망**.

#### 5.1-s ✅ A5-b — postponed 리전: **거부 하나가 세 가족을 덮고 있었고 셋의 크기가 48:6:0 이었다** · 92.88% → **93.64%** (2026-08-13)

⭐ **`a system task the tier-3 kernel refuses` 행(+54)의 내부 분포를 먼저 쟀다** — `$monitor`/`$strobe`
**48** · `$writemem*` **6** · `$dumpall`/`$dumpon` **0**. ⇒ 슬라이스는 앞의 하나이고, **`$dumpall`/
`$dumpon` 은 고쳐도 코퍼스에서 0 이므로 안 건드린다**(그 사실을 기록한다 — 조용히 자르지 않는다).

⭐⭐ **둘이 함께 거부돼 있던 이유가 곧 수정의 모양이다** — `dispatch` 의 `$monitor`/`$strobe` arm 은
**ExprId 와 메타데이터만 캡처하고 넷을 하나도 안 읽는다**(등록은 늘 store-독립이었다). store 에 묶인
것은 ⓐ 캡처된 인자의 **렌더**와 ⓑ `$monitor` 의 **변경 비교**뿐이고, 둘 다 `flush_postponed` 안에서
엔진의 store 를 읽었다. 네이티브 런에서 그 store 는 안 움직이므로 **`$monitor` 는 t0 establishment 줄을
찍고 그 뒤로 영원히 침묵**한다 — 진단도 크래시도 없이 출력만 사라진다.

두 부분이 필요했고 그래서 한 번에 열린다:

- **`flush_postponed_with<N>(nets)`** — 네 자리만 스레드했다(strobe 렌더 · monitor 렌더 · establishment
  seed · 변경 비교). `None` = 엔진의 store 이고 각 arm 이 원래 부르던 호출로 되돌아가므로 엔진 경로는
  **기계적 바이트 동일**(`dispatch_with`·`format_args_str_with`·`full_snapshot_with` 와 같은 모양).
- **tier-3 런 루프의 POSTPONED 리전** — 안정점(모든 리전 큐가 비고 cont-assign 이 fixpoint, **시간이
  아직 안 움직인** 자리)과 **세 종료 arm**(`$finish`/`$stop`/fatal)에서 부른다. 엔진 루프가 부르는 바로
  그 자리들이다. 인자 렌더가 범위 밖을 읽을 수 있으므로 **`drain_range_diags` 가 붙는다**(`propagate`
  와 같은 제3 생산자 문제).
- 빌림 분할은 `k_dispatch_systask` 의 것 — **커널이 아니라 아레나**를 넘긴다(커널은 `&mut Scheduler` 를
  들고 있어 둘을 동시에 못 빌려준다). 힙 넷과 호출은 한 층 아래 `HeapRouted` 가 답한다.

⭐ 앵커는 **iverilog 핀**이고 각 줄이 리전의 다른 부분이다 — establishment(무조건 출력) · 변경 재출력
**셋**(store-blind 비교가 잃는 바로 그 줄들: 엔진 store 에서는 값이 영원히 같아 `changed` 가 거짓) ·
`$strobe` 가 **정착값**을 본다는 것(`q=2 s=4`, 같은 줄의 `$display` 라면 `q=1 s=2`) · `$monitoroff`/
`$monitoron` 구간 · **`$display` 가 두 monitor 줄 사이에 오는 순서**(Active vs Postponed).

⚠️ **앵커가 `#2 $finish` 를 자기 슬롯에 따로 뒀다** — `$strobe` 와 **같은 슬롯의** `$finish` 는
pre-existing iverilog 발산이 있다(vvp 는 그 슬롯의 NBA 를 postponed 드레인 전에 적용, vita 는 안 한다).
**두 vita 백엔드가 동일**하므로 이 슬라이스 것이 아니고, 알려진 발산을 앵커 기대 출력에 넣으면 앵커가
앵커이길 그만둔다(§4.5.302).

⚠️⚠️ **거부 행을 핀하던 테스트 셋이 초록으로 공허해질 뻔했다** — 셋 다 `$monitor` 로 그 행을 재고
있었는데 그 형태가 이제 **돈다**. `$writemem*` 로 다시 철자했다(`run_json_reports_native_fallback_
on_a_refused_design` 은 이것이 **다섯 번째** 재선택이고, 그 churn 이 바로 그 테스트가 작동한다는
증거다). 그리고 `s1d4b2_…_refused_not_dispatched` 의 개수 단언 **6 → 4**.

**측정**: **5,956 / 6,361 = 93.63%**(+48 · 예측 +48) · 전 스위트 **5414 green** · **flip 런
5411/5414**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 7/7 사망**.

#### 5.1-t ✅ A2-ii — CRV: **한 줄이 표면 전체였다** · 93.63% → **94.13%** (2026-08-13)

⭐ **A2-i 이 CRV 를 남긴 이유가 정확히 옳았고, 정확히 한 줄이었다.** `class_randomize_run` 이 수신자를
`Scheduler::eval_ctx_top`(엔진의 넷)으로 읽었고, 네이티브 런은 그 store 를 t0 에 남기므로 **핸들이 0 으로
돌아와 `randomize()` 가 null 팔을 타고 0 을 반환하며 필드를 하나도 안 건드린다** — A1-ii 그대로.

⭐ **핸들 아래는 전부 공유다** — `class_heap` · 클래스 단위 테이블 넷(`class_rand`/`class_constraints`/
`class_dist`/`class_randc`) · 인라인 `with` 오버라이드 · RNG 가 모두 `SimState` 이고 두 커널이 빌린다
⇒ **draw·제약 풀이·필드 쓰기는 라우팅이 필요 없었다.** 그리고 그것은 주장이 아니라 계측이다:
`every_untreaded_store_read_in_builtins_sits_behind_a_reject_row` 가 `crv_draw.rs` 에서 raw read **4** 를
세는데 나머지 셋은 전부 `$writemem*` 것이다(그 핀은 이제 **3**).

두 번째 반쪽 = **status 쓰기**. `r = obj.randomize()` 의 결과를 `sched.resolve_lvalue_offsets` +
`sched.st.write_lvalue` 로 썼다 — funnel-OUTSIDE 쓰기이고, **A1-iii 가 지은 `TaskWrites` 싱크**에 그대로
올린다(`Direct` = 그 두 호출이라 엔진 경로는 기계적 불변). 오프셋도 리더로 푼다 — 오늘은 목적지가 bare
whole-net 이라 둘이 같지만, **한 store 의 lvalue 를 다른 store 의 리더로 푸는 것**이 목적지에 인덱스가
붙는 순간 틀어지는 모양이다.

⭐ **앵커가 값이 아니라 성질이다** — `iverilog 13` 은 제약 선언을 거부하므로(*"sorry: Constraint
declarations not supported"*) hand-IEEE 인데, `a = 14` 같은 draw 를 핀하면 **언어가 아니라 이 저장소의
LCG 를 핀하게 된다.** 대신 넷을 단언한다: **`ok=1`**(만족 가능한 제약에서 solver 는 성공해야 한다 —
store-blind 수신자 읽기가 여기서 0 을 낸다 = 1차 판별자) · **두 개의 서로 다른 `inside` 범위**(하나만
만족시킨 풀이도 보인다) · **`randc` 가 순열**(§18.6 — 2비트 필드 4 draw 가 0..3 을 한 번씩 · 균일
draw 는 앞의 둘을 통과하고 여기서 죽는다) · **status 가 목적지 넷에 착지**.

⚠️ **차분의 infeasible 케이스는 런타임에서 실패해야 한다** — 정적으로 모순인 `constraint { v > 2'd3; }`
는 **elaborate 가 E3009 로 거부**해서 백엔드에 도달조차 안 한다. 클래스 범위와 **교집합이 없는 인라인
`with`** 가 `class_randomize_run` 안에서 실패하는 형태다.

**측정**: **5,998 / 6,371 = 94.15%**(+32 · 예측 +32) · 전 스위트 **5416 green** · **flip 런
5413/5416**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 5/5 사망**.

⚠️ 그리고 이 슬라이스도 자기 게이트 테스트를 뒤집었다 — `a_randomize_call_is_refused_…` 는 **한 슬라이스
전에 정반대를 단언**하던 테스트다(A2-i 이 쪼갠 행을 A2-ii 가 지웠다). 이름과 주장을 함께 바꿨고,
raw-read 핀의 이유는 **§4.5.338 로 두 번째** 재작성이다(이름 대는 행이 쪼개지거나 열릴 때마다 문장은
거짓이 되는데 **숫자는 계속 통과한다**).

#### 5.1-u ✅ A3-iii — 위임 바디: **행이 `이름 부른다` 에서 `쓴다` 로 좁아졌다** · 94.15% → **94.50%** (2026-08-13)

⭐ **S3a 의 전제조건은 자기 실행기에 대해 정확했고, 그 실행기가 바뀌었다.** `Expr::Call` 로 닿는 평범한
함수는 `SimState` 의 `&self` 프레임 실행기에서 돌고 그것은 모듈 넷을 **엔진의 flat store** 에서 읽으므로,
그런 바디를 admit 하면 exit 0 에 t0 값을 읽는다 — 그래서 *"자기 창 밖 넷을 이름 부르지 않는다"* 였다.
A3-ii-a 가 **구동** 반쪽에 대해 이미 반증했고(워크가 직접 도는 태스크는 커널로 읽는다), 이것이 **위임**
반쪽이다: 그 실행기에 **호출자의 store 를 넘긴다**.

⭐ **composite 가 일을 한다** — `eval_ctx_with_reader` 가 `HeapRouted` 로 감싸므로 **프레임 슬롯은
`self`(활성화 창)로, 모듈 넷은 아레나로** 갈린다. V1 슬라이스 2 가 힙을 위해 지은 바로 그 래퍼다.
스레드한 자리는 넷뿐 — rhs 평가 · **분기 조건**(rhs 만 스레드하면 놓치는 별도 사이트) · lvalue 인덱스 ·
프레임 안 `$display` 렌더. `None` 은 엔진 자신의 넷이고 각 사이트가 원래 부르던 호출로 되돌아간다.

⚠️⚠️ **쓰기는 스레드로 안 된다.** 그 바디의 모든 목적지는 `SimState::frame_write_lvalue` 를 지나는데
그것은 이 state 의 `&self` 이고 **호출자의 아레나에 닿을 방법이 없다** — 모듈 넷에 대입하면 죽은 store 에
조용히 착지한다. ⇒ **행을 지우지 않고 좁혔고**, 그 결정은 좁히기 **전에** 쟀다: 이 행이 막던 26 중
**22 가 창 밖을 읽기만 하고 4 가 쓴다.**

⚠️⚠️ **그리고 거부 설계를 짓는 데 프로브가 필요했고 그 답이 기록할 값어치다** — 모듈 넷에 대한 평범한
`g = g + 1` 은 **한 단계 앞에서 거부된다**(elaborate E3009 — *"an assignment to a net outside the
function … is outside the frame-call subset"*). 이 행에 실제로 닿는 것은 **클래스 필드 쓰기**
`c.v = …` 이고, 그 lvalue 청크가 모듈 스코프 **핸들 넷**을 이름 부른다. 뻔한 소스 형태로 지었으면
**elaborate 가 거부해서 통과하는 테스트**가 됐을 것이다.

⚠️ **다섯 테스트가 옛 행을 핀하고 있었고 넷은 형태를 바꿔 이빨을 되찾았다**(else 팔 · 루프 바디 ·
선언 순서 · 게이트 AND · run.json 모양). 다섯째는 그럴 수 없어서 **은퇴시키고 이유를 적었다** —
`LvalChunk::width` 엣지(비상수 part-select 의 msb)는 이제 이 행에 **도달 불가**다: 읽기는 거부하지
않고, 쓰기 쪽은 `c.v[i:0] = …` 이 그 자체로 pre-existing elaborate 갭(E3010)이다. **조용히 지우지 않고
"도달 불가" 로 기록**한다 — 둘 중 하나가 닫히면 테스트가 빚이라는 것을 그 주석이 말한다.

⚠️⚠️ **그리고 행을 지우자 그 아래 pre-existing silent-wrong 이 드러났다 — flip 런만 잡았다.**
S3a 의 행은 **위임 실행기 둘을 한꺼번에** 덮고 있었다: `run_frame_call`(식 호출)과 **`run_task`**(A3-i
subset 경로 = output formal 을 가진 함수의 `Terminator::Call`). 앞의 것만 스레드하고 행을 열자 뒤의 것이
엔진의 안 움직이는 store 를 계속 읽었다 — `while (getnext(i, v) == 1)` 이 모듈 배열에 대해 첫 호출에서
0 을 돌려주고, **루프 바디가 한 번도 안 돌고 마지막 줄만 찍혔다**(exit 0 · 진단 없음). 그 설계의 테스트는
기본 백엔드로 돌기 때문에 **전 스위트가 초록이었다.** ⇒ **이 저장소가 같은 것을 세 번째로 쟀다**(V1
슬라이스 2d · A2-i · 여기). `run_task`/`run_subset_task` 도 같은 방식으로 스레드.

⚠️⚠️ **판별자를 지었더니 그것이 또 두 자리를 더 찾았다** — 첫 배터리에서 `frame lvalue INDEX` 와
`subset 바디의 분기`가 생존했다. 눈먼 축이라 설계를 지었고(프레임-로컬 **배열**을 **모듈 넷 인덱스**로
쓰고, 모듈 넷으로 **분기**), 그러자 **뮤테이션이 아니라 진짜 발산**이 나왔다: 쓰기 인덱스가
`frame_or_class_write` 와 `frame_write_lvalue` 의 **원소 read-modify-write** 두 군데에서 아직 엔진의
store 를 풀고 있었다 ⇒ `loc[sel] = src[fd]` 가 `loc[0]` 에 착지하고 `v=0 v=0 v=0` 을 찍었다.
**"라우팅은 한 군데가 아니다" 의 네 번째 실증**이고, 이번엔 **쓰기-인덱스** 축이다.

⚠️ 절차 사고 하나: **배터리가 도는 중에 트리를 고쳤다.** 배터리의 `restore()` 가 자기 스냅샷(=배터리
시작 시점)을 되쓰므로 그 사이 편집이 통째로 되돌아갔고, 뮤테이션 하나가 트리에 남은 채 빌드가 깨졌다.
LOOPROMPT §4 가 이미 적어 둔 규칙의 재발 — **배터리와 편집은 겹치면 안 된다.**

**측정**: **6,021 / 6,373 = 94.48%**(+23 · 예측 +22) · 전 스위트 **5419 green** · **flip 런
5416/5419**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 10/10 사망**.

#### 5.1-v ✅ A3-iv — `has_hier_call`: **거부의 이유가 자기 단계(phase)에 관한 것이었다** · 94.48% → **94.79%** (2026-08-14)

⭐⭐ **행이 대던 이유는 참인데 *다른 단계*에서 참이었다.** *"deferred 계층 enable 의 `Call.target` 은
finish-phase resolve 전까지 placeholder 라서 `frame_suspends` 가 뚫고 볼 수 없다"* — 이것은
**elaborate 안에서** 참이고, `force_suspend` 가 존재하는 이유가 바로 그것이다(그쪽
`compute_suspendable_tasks` 는 패치 **전에** 돈다). 이 술어는 `simulate` 에서, 즉 **패치 후에** 돈다.

계측이 그것을 확인했다 — 이 행에 도달하는 모든 설계에 대해 워크를 계기화하니 **해결 안 된
`Call.target` 이 0개**다. ⇒ 행이 대신 서 있던 질문은 답할 수 있고, **바로 위 행이 이미 그것을 묻는다.**

⭐ **지우기 전에 네 형태를 쟀다**(§5.1-p 의 `class_virtual` 선례): 단순 hier enable · **park 하는 hier
callee** · **output formal** 이 인스턴스 경계를 넘는 copy-out · **인스턴스 둘**. 넷 다 옳고, 둘째는
**`frame_suspends` 가 거부한다**(*"a task frame that SUSPENDS"*) — 그 walk 의 `None` 팔이 여전히
**fail-closed** 이므로 언젠가 정말 안 풀린 타깃이 오면 그때는 그것이 잡는다. 바뀐 것은 **오늘 그런 것이
없다**는 사실이다.

⚠️ **이 파일이 `has_hier_call` 을 세 번 다르게 적었다** — S3a *"dead"* → A3-ii-a *"LIVE, 19 설계"* →
여기 *"gone"*. §4.5.338 의 세 번째이자 가장 명확한 형태: **거부 행의 이유가 "언제" 에 관한 주장이면,
다음 단계가 그것을 무효화한다.**

**측정**: **6,041 / 6,374 = 94.78%**(+20 · 예측 +19) · 전 스위트 **5421 green** · flip 런
(실패 3 = 백엔드 이름 핀) · **발산 0**.

⚠️ **뮤테이션 셋 다 생존이고 셋 다 이유가 다르다, 그리고 그것을 쟀다** — ⓐ **전이 arm** 은 A3-i 이
이미 *"오늘 중복"* 으로 기록한 그것이다(`frames_admitted` 가 **모든** func 에 묻기 때문에 hier callee
자신의 행이 먼저 잡는다 · 이번 슬라이스가 그 기록을 재확인했다) · ⓑ **`None` fail-closed 팔** 은
**도달 불가**다(계측: 미해결 타깃 0) · ⓒ `Delay` 를 park 으로 보고하는 줄은 **A3-ii-a 배터리의 케이스
D 가 이미 죽였다**(`Fork => return false`). 셋 다 **"덮였다" 가 아니라 각각의 이유로** 기록한다.

#### 5.1-w ✅ A8-b — deferred assertion: **기계장치는 이미 공유였고 없던 것은 두 리전이었다** · 94.78% → **95.03%** (2026-08-14)

⭐ **A5-b 와 같은 모양이고, 이번엔 더 극단적이다.** §16.4.3 은 deferred action 의 텍스트를 **REACH 시점에**
렌더한다 — 큐에 들어가는 것은 이미 `String` 이다. 따라서 **`mature_deferred` 는 넷을 하나도 안 읽고**,
store 에 묶인 줄은 `try_defer` 안의 렌더 **하나**인데 그것은 `dispatch_with` 가 S1d-4b 이래 스레드해 왔다.

⇒ tier-3 에 없던 것은 **OBSERVED·REACTIVE 두 리전의 자리**뿐이다. 엔진 cascade 가 부르는 곳에 넣었다:
timestep 의 Active/Inactive/NBA 가 비고 **postponed 드레인 전에**, Observed 먼저 Reactive 다음 · 성숙한
리포트가 프로세스를 깨울 수 있으므로 각각 뒤에 **propagate + cascade 재진입**(엔진이 `continue` 하는
이유) · 세 종료 arm 에 `drain_deferred_on_finish`.

⚠️ hand-IEEE 다 — `iverilog 13` 이 deferred assertion 을 통째로 거부한다(*"sorry: Deferred assertions
are not supported"*).

**측정**: **6,064 / 6,381 = 95.03%**(+16 · 예측 +16) · 전 스위트 **5424 green** · **flip 런**(실패 3 =
백엔드 이름 핀) · **발산 0** · **뮤테이션 5/5 사망**.

⚠️⚠️ **첫 배터리의 생존 하나가 앵커의 눈먼 축이었다 — `assert final` 이 한 번도 실패하지 않았다.**
`q < 4'd3` 은 참이라 Reactive 큐가 늘 비어 있었고, **Reactive 성숙을 통째로 지우는 뮤테이션이 전 스위트를
통과**했다. `q < 4'd1` 로 바꾸자 두 엣지에서 리포트가 나고 **순서**(`R q=1` → `O q=2` → `R q=2`)가 두
큐가 각자의 리전에서 드레인된다는 것을 보인다. **"두 큐가 있다" 는 둘 다 뭔가를 낼 때만 판별자다.**

⚠️ 그리고 이 슬라이스는 **SVA 거부 테이블의 케이스 하나를 통째로 지웠다** — `sva_shapes_that_need_
machinery_still_refuse_by_their_own_name` 의 두 케이스 중 deferred 쪽이 이제 돈다(개수 단언 **2 → 1**).

#### 5.1-x ✅ #1 clocking — **테이블은 이미 공유였고, 없던 것은 두 끝과 한 자리였다** · 95.03% → **95.29%** (2026-08-14)

⭐ **또 하나의 보수적 거부, 그리고 이번엔 "기계장치가 없다" 가 세 조각으로 갈렸다.** 클로킹 블록은
런타임 기계장치가 아니다 — elaborate 가 아이템마다 **홀딩 넷**(`__clk_*` / `__clkout_*`)을 만들어
`cb.sig` 를 거기 alias 하고, **Null 바디의 마크된 `always @(clk);` 핸들러**를 낸다. 엔진에 도착하는
것은 평범한 IR + 세 사이드카(`clocking_inputs`/`clocking_commit`/`clocking_outputs`)이고, 그 셋과
`preponed_buf` 는 **전부 `SimState`** = 두 커널이 빌리는 같은 객체(`dyn_heap` 과 정확히 같은 상황).

store 에 묶여 있던 것은 **두 끝**이다:

| 자리 | 무엇을 읽고/쓰나 | 미스레드일 때 |
|---|---|---|
| `snapshot_preponed` | 소스 넷을 **읽는다** | 네이티브에선 엔진 슬롯(선언 초기값)을 영원히 샘플 |
| `clocking_commit_plan` OUTPUT | 홀딩 넷을 **읽는다** | 같은 이유로 X |
| `commit_clocking_sample` | 홀딩/소스 넷에 **쓴다** | 네이티브가 안 읽는 store 로 커밋 = `cb.sig` 가 영원히 X |

⭐ **plan / apply 로 쪼갠 이유가 정확성 논거다.** 두 store 의 쓰기 지점이 다르므로 **결정**(어떤 넷을
어떤 값으로, 어떤 순서로)만 공유하고 쓰기는 각자 한다. 쓰기를 읽기 뒤로 미루는 것이 관측 불가인 것은
가정이 아니라 **테이블의 성질**이다 — INPUT 은 홀딩 넷만 쓰고 OUTPUT 은 홀딩 넷만 읽는데, elaborate 는
**아이템마다 새 홀딩 넷**을 만들고 한 아이템의 방향은 input XOR output 이다(`ClockingDir::Inout` 은
loud reject). A1-iii 의 `TaskWrites::Collect` 가 기대는 논거와 같은 모양이고, **뮤테이션 K(두 페이즈
순서 뒤집기)의 생존이 그 성질의 증명**이다.

⭐ **세 번째 조각은 테이블이 아니라 자리다.** 엔진은 커밋을 `propagate_changes` 패스 (a) **안에서**,
fire/busy/self-write 검사 뒤 **멀티넷 dedup 앞에서** 하고 `continue` 한다. tier-3 의 `WakeTable::wake`
가 정확히 그 지점에서 핸들러를 `clocked` 로 우회시킨다(dedup 앞이라 두 넷에서 발화하면 엔진처럼
두 번 커밋한다 · `seen` 에도 안 들어간다).

⚠️⚠️ **flip 런이 진짜 발산을 잡았고, 그것은 엔진의 pre-existing 결함이었다.** `@(posedge cb.d)` 를
t=8 에 arm 한 설계가 VM 에선 **즉시 발화**(t=8)하고 native 에선 정지했다. 원인: **커밋은
`propagate_changes` 가 자기 changed set 을 이미 가져간 뒤에 일어나므로 그 변경은 "다음 propagate"
몫인데, 엔진의 timestep 에는 다음 propagate 가 없다** — 루프가 break 하고 POSTPONED 가 돌고 시간이
넘어간 **다음 슬롯**에서야 쓸리면서, **자기가 일어난 슬롯의 `slot_edge` 를 들고 도착한다**. tier-3 의
루프는 deferred 리전 cascade 때문에 안정점에 propagate 를 이미 갖고 있어서 처음부터 옳았다. 수정 =
엔진 안정점에 **`dirty` 가 비지 않았으면 한 번 더 propagate**(늦은 생산자가 하나뿐이므로 나머지
코퍼스는 바이트 동일). ⇒ **native 가 옳았고 사다리가 올라갔다.**

⚠️ **그 결함을 핀하던 테스트가 자기가 지키려던 것을 반대로 적고 있었다** —
`clocking_holding_net_edge_still_fires` 는 `d = 1` 을 t=0 에 두고 t=8 에 arm 했으므로 **유일한 posedge
가 wait 이 생기기 전(t=5)에** 있었다. 통과한 이유가 곧 결함이었다. 설계를 t=15 에 진짜 엣지가 오도록
고쳤고, **평범한 넷으로 같은 모양을 지으면 iverilog 도 정지한다**는 것이 옛 기대가 틀렸다는 근거다.

⚠️ **오라클 둘 다 앵커가 못 된다.** `iverilog 13` 은 `clocking` 을 **파싱조차 못 한다**(*syntax
error*). `verilator 5.050` 은 지원하지만 **Observed 리전에서 샘플**하므로 Active 리전의
`always @(posedge clk)` 가 **직전 엣지의** 샘플을 읽는다(`cb.d` = 0,1,2 vs vita 1,2,3). vita 는 엣지
**감지 시점**에 커밋하는 단순화 모델(엔진의 documented hand-IEEE)이고 세 백엔드가 그것을 공유하므로,
verilator 의 숫자를 핀하면 **vita 가 구현하지 않는 모델을 핀**하게 된다 ⇒ 앵커는 hand-IEEE.

⚠️⚠️ **하네스 갭 여덟 번째** — `build_with_opts` 에 세 테이블이 없었다. 없으면 핸들러는 **아무것도
안 하는 Null 바디 프로세스**이고 `cb.sig` 는 X 로 앉아 있어 **두 백엔드가 아무도 안 한 샘플에
일치**한다. 이번에도 실패가 아니라 **그 파일의 앞선 일곱 주석을 읽어서** 찾았다.

⚠️ **`wake.rs` 가 적어 둔 "GATE DEPENDENCY" 를 실측이 반증했다** — *"엔진은 `last_blocking_writer` 를
propagate 시점에 LIVE 로 읽고 여기는 changed 튜플의 스냅샷을 읽으므로, 클로킹이나 force 를 받아들이면
등가성이 깨진다"*. 엔진도 스냅샷한다(`edges.extend(…)` 프롤로그 = `take_changed` 와 같은 지점).
**§4.5.338 재발이고, 이 이유는 애초에 참인 적이 없었다.**

**측정**: **6,091 / 6,392 = 95.29%**(+16 · 예측 +16) · 전 스위트 **5427 green** · **flip 런**(실패
3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 9/11 사망**.

⚠️ **생존 둘은 등가이고 둘 다 쟀다** — ⓐ `store_sample_words` 의 **최상위 워드 마스크**는 도달
불가다(홀딩 넷의 폭은 elaborate 가 소스에서 그대로 복사하므로, 들어오는 `Value` 는 목적지 폭으로 이미
마스크돼 있다 · §4.5.332 의 `resize_word` 폭-0 가드와 같은 클래스) · ⓑ **두 페이즈의 순서**는 위의
disjoint-net 성질 때문에 관측 불가 — **이 생존이 plan/apply 분리의 정당성 증명**이다.

⚠️ **첫 배터리의 생존 하나는 등가가 아니라 내 차분 설계의 구멍이었다** — `reg clk = 1'b1;` 에 드라이버가
없어 **posedge 가 한 번도 안 일어났고**, 아무것도 커밋되지 않는 행이 t0 seed 를 지우는 뮤테이션을
통과시켰다. **엣지를 만들지 않는 클로킹 행은 아무것도 재지 않는다.**

#### 5.1-y ✅ #2 force/release — **세 조각 중 하나만 진짜로 없었다** · 95.29% → **95.53%** (2026-08-14)

행이 댄 이유는 *"v1 에 force 기계장치가 통째로 없다 — 넷마다의 force 플래그·연속 재평가·assign/
deassign 약순위"* 였다. 세 조각을 각각 재니 **둘은 이미 공유**였다:

| 조각 | 실측 |
|---|---|
| 넷마다의 force 플래그 | `SimState::forced` — **한 테이블**. 미러링이 아니라 아레나 퍼널에 **스레드**했다(`&[bool]`) |
| §9.3.1/§9.3.2 레지스트리 | `active_forces`·`latent_assigns`·감도 사이드카·`assign_ranks` — 전부 `SimState` |
| **연속 재평가 fixpoint** | **이것만 없었다** |

⭐ **결정과 쓰기를 갈랐다**(슬라이스 #1 과 같은 모양). `force_prologue`/`force_epilogue` ·
`release_prologue`/`release_epilogue` · `force_keys_for`/`force_entry` 가 전부 **레지스트리만** 읽고
쓴다 — 넷 값을 하나도 안 만지므로 두 번째 커널에 넘겨도 엔진 store 가 샐 수 없다(A1-ii 가 좁은 seam 을
고집한 이유의 반대편 증명). 엔진의 `k_force`/`k_release`/`reeval_active_forces` 를 **그 helper 로 다시
쓰고**, tier-3 이 같은 fixpoint 를 자기 store 에 대해 돈다 — 갈리는 것은 **eval·write·dirty seed** 셋뿐.

⚠️ 플래그는 **퍼널 안에서 chunk 마다** 본다(엔진의 자리 그대로) — `{a, b} = x` 에서 `a` 만 forced 면
청크 하나만 떨어진다. 빠른 경로(`write_chunk_word`)에도 같은 게이트가 필요하다(`eval_store_word` 가
그 메서드에 직행한다).

⚠️⚠️ **pre-existing silent-wrong 하나를 두 오라클이 부른다 — 고쳤다.** `release w` (구동되는 wire)가
드라이버로 **복귀하지 않았다**: 플래그를 끄는 것은 넷을 안 움직이므로 dirty-driven settle(§4.5.335)이
그 cont-assign 을 다시 안 돌린다 ⇒ forced 값이 **그 assign 의 입력이 우연히 움직일 때까지 살아남는다**
(iverilog·verilator `3` / vita 세 백엔드 전부 `240`). 수정 = `release` 가 목적지를 구동하는 assign 을
**다시 dirty 로** 찍는다(`drivers_of_net`, 두 store 각자의 worklist).

⚠️⚠️ **소스 스캔 핀 하나가 줄바꿈에 눈멀어 있었다.** `every_tier3_store_goes_through_the_one_write_
funnel` 은 "`arena.write_lvalue(` 를 포함한 줄" 을 세는데, rustfmt 가 퍼널이 **아닌** 유일한 사이트에서
`k.arena` 와 `.write_lvalue(` 를 두 줄로 갈라 놨다 ⇒ **핀은 "정확히 하나" 라고 읽으면서 둘이었다.**
슬라이스 #2 가 인자를 하나 더해 줄이 붙는 바람에 드러났다. 그 사이트는 퍼널로 돌리고, 스캔은
**주석 제거 + 3줄 조인 후** 매칭한다 — 포매터가 테스트의 이빨을 정할 수 없게.

⚠️ **`gate_refused!` 사이트가 하나 남았다** — `k_disable_fork`. `s1d4a_refused_workers_are_loud_not_
silent` 는 두 슬롯이었고 둘째는 A1 을 따라 세 번 옮겨 다녔으며 첫째가 `force` 였다. A4 가 `disable
fork` 를 가져가면 이 테스트는 **주제가 없어진다** — 그때는 설계를 지어내지 말고 이유를 적고 은퇴시킨다.

**측정**: **6,118 / 6,404 = 95.53%**(+15 · 예측 +15) · 전 스위트 **5431 green** · **flip 런**(실패 3 =
백엔드 이름 핀) · **발산 0** · **뮤테이션 11/11 사망**.

⚠️⚠️ **첫 배터리 생존 넷이 전부 내 설계의 눈먼 축이었고, 그중 둘의 이유가 이 슬라이스 고유하다** —
**연속 재평가가 켜져 있으면 새어 나간 쓰기를 다음 re-pin 이 고쳐 준다**. `#` 뒤에서 관측하면 아레나의
force 게이트를 통째로 지워도 값이 맞는다. 고칠 수 없는 것은 **그 사이에 일어난 일**이다: 지연 없는
`$display`, 그리고 엣지 프로세스가 이미 센 posedge. 나머지 둘은 단순한 커버리지 구멍이었다 —
release 를 **구동되는 wire** 에 거는 행이 하나도 없었고(엔진 절반 미측정), **force 가 먼저이고 assign 이
나중**인 순서가 없었다(공유 코드라 차분은 원리적으로 못 본다 → 절대 앵커).

⚠️ 앵커가 **둘로 쪼개진다** — 상수 force·release 스냅백·구동 억제는 **iverilog 핀**이고, 연속 재평가와
assign/deassign 순위는 **hand-IEEE** 다(iverilog 가 *"sorry: procedural continuous assignments are not
yet fully supported. The RHS … will only be evaluated once"* 라고 자인하고 `101` 을 낸다 — vita 가 앞선
두 번째 항목).

⚠️ **하네스 갭 아홉 번째** — `build_with_opts` 에 `assign_ranks` 가 없었다. `force`/`assign` 과
`release`/`deassign` 은 **같은 두 IR 문장**이고 그 사이드카가 유일한 판별자다 ⇒ 없으면 절차적 `assign`
이 **강한 force 로 실행**되고 두 백엔드가 사이좋게 일치한다.

#### 5.1-z ✅ #3 frame stmt drop — **census 가 행을 두 조각으로 갈랐고 둘 다 이미 실행되고 있었다** · 95.53% → **95.78%** (2026-08-14)

행의 문구는 *"a subroutine statement the frame executor drops"* 였다. **어떤 문장인지는 안 적혀 있어서
계측했다** — 이 행에 닿는 모든 설계에서:

| 문장 종류 | 설계 수 |
|---|---|
| `SysTask::DynNew`(= `d = new[n]`) | **13** |
| `Disable`(scope) | **2** |
| 그 밖의 어떤 종류든 | **0** |

⭐ **둘 다 `run_frame_call` 이 이미 실행하는 arm 이다** — `DynNew`/`DynDelete` 는 `frame_dyn_new`/
`dyn_heap` 직접 조작이고, `Disable` 은 elaborate 가 마커 + 형제 `Goto` 로 낮추므로 실행기의
`_ => {}` 가 **드롭이 아니라 그 마커의 올바른 실행**이다(§4.3 이 module 경로에 대해 이미 적어 둔 것).

⚠️⚠️ **하지만 순전히 보수적이지는 않았다 — 밑에 진짜 결함이 하나 있었다.** `frame_dyn_new` 가 크기
인자를 **`mk_eval_ctx`(엔진의 넷)로** 읽는다. 네이티브 런에서 그 store 는 안 쓰이므로 모듈 넷을 크기로
쓰는 프레임 로컬 `d = new[n]` 이 **엔진 슬롯의 값**으로 할당된다 — V1 슬라이스 2c 가 module 경로에서
잰 바로 그 결함(`size=0`)이고, 이번엔 `&self` 프레임 실행기를 통해 도달한다. `frame_dyn_new_with` 로
스레드.

⭐ **앵커의 판별자는 한 줄뿐이다** — `n` 의 선언 초기값은 **엔진 슬롯에도** 있으므로 첫 호출은 두
store 가 우연히 같은 답을 낸다(`A 508`). `n = 8` 뒤의 **두 번째 호출**만 갈린다(`B 821` vs 미스레드
`508`). **초기값이 같은 설계는 첫 호출로 아무것도 재지 못한다.**

**측정**: **6,144 / 6,415 = 95.78%**(+15 · 예측 +15) · 전 스위트 **5433 green** · **flip 런**(실패 3 =
백엔드 이름 핀) · **발산 0** · **뮤테이션 4/5 사망**.

⚠️ **생존 1 은 등가가 아니라 도달 불가이고 쟀다** — `DisableKind::Fork` 를 함수 바디 안에 두는 두 철자
(`fork … join_none disable fork;` · 맨 `disable fork;`)가 **둘 다 elaborate E3009** 다. 가드는 남긴다
(fail-closed)고 그 사실을 적는다.

⚠️ `DynDelete` 는 **코퍼스 설계 0** 이다. 같은 실행기 arm·같은 가족이라 함께 받아들였고(반쪽만 거부하면
행의 이유가 거짓이 된다) 그것을 이득으로 세지 않고 기록한다 — `$dumpall`/`$dumpon` 선례의 반대 방향
판단이며, 차이는 **여기서는 거부를 유지하는 데도 코드가 든다**는 것.

#### 5.1-aa ✅ #4 file_directed — **행 하나의 기계장치가 fd 읽기 하나였다** · 95.78% → **95.90%** (2026-08-14)

`$fmonitor`/`$fstrobe` 는 **동결된 `Monitor`/`Strobe` 태스크 id 를 재사용**한다 — `args[0]` 이 디스크립터
라는 것을 말해 주는 것은 `file_directed_stmts` 사이드카 하나뿐이다. 나머지는 **전부 이미 공유**였다:
캡처는 `SimState::postponed`(A5-b 의 리전) · 파일 테이블은 `SimState`(A1-iv-b) · 렌더는 A5-b 가 스레드한
`flush_postponed_with`.

⇒ store 에 묶여 있던 것은 **`split_file_directed` 의 fd 평가 한 줄**(맨손 `sched.eval`)이다. **A5-a 와
같은 모양이고 이유의 문구까지 같다** — 미스레드 fd 읽기는 네이티브에서 엔진 슬롯의 값을 돌려주므로,
모니터가 **설계가 연 적 없는 디스크립터**를 향하게 된다.

⭐ **앵커의 판별자는 fd 가 넷이라는 것뿐이다** — 리터럴 디스크립터는 상수라 두 store 가 같은 답을 낸다.
앵커는 `$fopen` 의 반환을 `integer fd` 에 담고 **파일 내용**을 iverilog 값으로 핀한다(+ `run.json` 이
`native` 라는 anti-vacuity — 거부되면 VM 으로 떨어져 위의 기존 테스트들과 같은 것을 재게 된다).

**측정**: **6,154 / 6,417 = 95.90%**(+10 · 예측 +8) · 전 스위트 **5435 green** · **flip 런**(실패 3 =
백엔드 이름 핀) · **발산 0** · **뮤테이션 3/3 사망**.

⚠️⚠️ **배터리가 SURVIVED 를 거짓으로 보고했고, 원인은 테스트 필터였다.** 케이스 C(무효 디스크립터가
`args[0]` 를 소비하지 않게)는 `-E 'test(fmonitor) or test(fstrobe) or test(file_directed) or
test(monitor)'` 로 돌렸는데, **그것을 죽이는 테스트의 이름이
`an_invalid_descriptor_is_loud_and_writes_nothing`** 이라 넷 중 어느 패턴에도 안 걸렸다. 손으로 걸어
확인하니 즉시 갈린다(`W4022` vs stdout 폴백) ⇒ **배터리의 필터가 킬러보다 좁으면 SURVIVED 는 정보가
아니라 잡음이다.** 필터를 바이너리 단위(`binary(file_monitor_strobe)`)로 넓혀 재실행.

⚠️ 그 케이스는 이 슬라이스의 것이 아니라 **`split_file_directed` 의 pre-existing 절**이었고, 그것을
지키는 테스트도 반쪽이었다 — 기존 행이 쓰던 `32'hdead_beef` 는 **읽을 수 있는 수**라
`unwrap_or(u32::MAX)` 에 도달조차 안 한다. X/Z 디스크립터 행을 지어 덮었다.

⚠️ **하네스 갭 열 번째** — `build_with_opts` 에 `file_directed_stmts` 가 없었다(없으면 `$fmonitor(fd, …)`
가 **fd 를 값으로 찍는 평범한 `$monitor`** 이고 두 백엔드가 일치한다).

⚠️ `dispatch.rs` 의 raw-read 핀이 **1 → 0** 이 됐다. 지우지 않고 0 으로 유지한다 — 이 핀의 일은 **새
raw read 가 생기는 것을 알아채는 것**이고, 지금 하나도 없는 파일이야말로 하나가 몰래 들어가기 가장 쉬운
자리다.

#### 5.1-ab ✅ #5 `final` blocks — **"restate the post-loop drain" 이 세 줄이었다** · 95.90% → **96.04%** (2026-08-14)

행의 문구가 곧 할 일이었다: `Scheduler::run_finals` 는 `final_procs` 의 바디를 하나씩 돌리고 그 뒤에
postponed 를 flush 하는 것이 전부이고, tier-3 은 **둘 다 갖고 있었다** — 바디는 `dispatch_body`(리전
루프가 쓰는 그 실행기 선택)이고 flush 는 A5-b 가 스레드한 `flush_postponed`. 새 기계장치 0.

빠져 있던 **두 번째 반쪽이 요점**이다: **`arm_t0` 에 `final_procs` skip 이 없었다.** `final` 은 IR 에서
Initial 모양이므로 그 행이 없으면 **t0 에 평범한 initial 로 큐잉**된다 — 앵커의 첫 줄이 그것을 잡는다.

⭐ **position 은 `done`** — 엔진은 `Scheduler::run` 이 리턴한 **뒤에**, 그것이 낸 모든 finish reason 에
대해(델타 리밋 포함) 부르고, **t0 settle 이 실패했을 때만** 안 부른다. `done` 이 그 하나를 뺀 모든 exit 의
퍼널이라 양쪽이 한 번에 재현된다.

⭐⭐ **곁가지로 SVA 두 형태가 함께 열렸다** — `cover property` 와 liveness 는 end-of-sim 의무 검사를
`final_procs` 에 등록하므로 이 행 하나가 유일한 거부자였다. **지우기 전에 쟀다**(A2-i 의 `class_virtual`
규칙): covered 런에서 `hits: 2`, uncovered 에서 `hits: 0`, 세 백엔드 동일 ⇒ 두 거부 핀을 **positive
테스트로 뒤집었다**(슬라이스 3a 의 패턴).

⚠️⚠️ **그 결과 `sva_shapes_that_need_machinery_still_refuse_by_their_own_name` 의 케이스가 0 이 됐다.**
V1 슬라이스 1 이 둘로 시작했고 A8-b 가 하나(deferred)를, 이것이 마지막 하나를 가져갔다. **지우지 않고
개수 단언 `== 0` 으로 남긴다** — 다음에 행을 여는 슬라이스가 이웃 핀을 새로 짓거나 없는 이유를 적게 하는
표시다. `s1d4c2c_each_refusal_row_has_a_design` 도 4 → 3.

**측정**: **6,177 / 6,432 = 96.04%**(+8 · 예측 +8 · SVA 두 형태는 덤) · 전 스위트 **5438 green** ·
**flip 런**(실패 3 = 백엔드 이름 핀) · **발산 0** · **뮤테이션 4/5 사망**.

⚠️ **생존 1 은 등가이고 쟀다** — `done` 안에서 `run_finals` 와 `drain_range_diags` 의 순서를 뒤집어도
같다: `run_finals` 는 바디마다 `flush_postponed` 로 끝나고 그것이 `drain_range_diags`(= VCD drain)를
부른다. finals 가 없으면 즉시 리턴하므로 순서가 무의미하고, 있으면 안에서 이미 드레인된다. **VCD 를 여는
차분 행을 지어 확인한 뒤** 등가로 기록.

⚠️ **앵커에 알려진 발산 하나를 명시했다** — `final` 안의 `$strobe`(`F2S`)를 vita 는 찍고 iverilog 는 안
찍는다. 엔진의 기존 선택(*"end-of-sim 은 마지막 timestep"*)이고 세 백엔드가 공유한다. ⚠️ `$finish` 를
클럭 엣지에서 **일부러 비켜 놨다** — Active 리전 `$finish` 가 대기 중인 엣지 프로세스를 드레인하지 않는
것은 `run_finals` 가 이미 적어 둔 pre-existing 한계이고, 엣지에 맞추면 **이 슬라이스가 아니라 그 한계를
핀하게 된다**.

#### 5.1-ac ✅ #6 stage — **행이 잘못된 이웃과 한 문장에 묶여 있었다** · 96.04% → **96.14%** (2026-08-14)

`probe` 와 `stage` 는 게이트에서 **한 주석을 공유**했다 — *"G2 rail 들은 인터프리터의 change hook 을
탄다"*. **`probe` 에 대해서는 참이고**(`emit_probe_change` 가 `note_change` 안에서 불린다) **`stage` 에
대해서는 거짓이다**: `$vita_stage` 는 hook 이 아니라 **명시적 호출 사이트**이고, elaborate 가 no-op
`Display` + StmtId 집합으로 낮춘다. 레일의 상태(`stage_lines`·`stage_idx`·`stage_enabled`)는 전부
`SimState` = 두 커널이 빌린다.

⇒ store 에 묶인 것은 **`run_vita_stage` 의 인자 읽기 둘**(레이블 · 값 목록, 맨손 `sched.eval`)뿐이다.
미스레드면 네이티브 런의 `stage.jsonl` 이 **엔진의 안 건드린 슬롯**을 기록한다 — 없는 것이 아니라 조용히
틀린 G2 산출물이고, **A7 의 `coverage.json` 과 정확히 같은 실패 모드**다.

⭐ **앵커의 모든 부분이 판별자다** — 레이블이 **넷에서** 오고(리터럴 레이블은 상수라 두 store 가 같은
답을 낸다) 값도 넷이며 **둘 다 두 호출 사이에서 움직인다** + `run.json` anti-vacuity.

**측정**: **6,185 / 6,433 = 96.14%**(+7 · 예측 +7) · 전 스위트 **5440 green** · **flip 런**(실패 3 =
백엔드 이름 핀) · **발산 0** · **뮤테이션 4/4 사망**.

⚠️ `render.rs` 의 raw-read 핀이 **2 → 0**. `dispatch.rs`(#4)에 이어 두 번째로 0 이 된 파일이고, 같은
이유로 지우지 않고 0 을 유지한다. ⚠️ **하네스 갭 열한 번째** — `build_with_opts` 에 `stage_stmts` 가
없었다(없으면 `$vita_stage` 가 레이블과 값을 **stdout 에 찍는 평범한 `$display`**).

#### 5.1-ad ✅ #7 frame body suspends — **네 절 중 셋은 도달 불가였고, 남은 하나는 실행기 둘 중 하나만의 이야기였다** · 96.14% → **96.26%** (2026-08-14)

행의 문구는 *"a subroutine body that suspends, forks or calls a task"* — **네 개의 절**이다. 계측:

| 터미네이터 | 설계 수 |
|---|---|
| `Terminator::Call`(중첩 호출) | **7** |
| `Delay` · `Wait` · `Fork` | **0** |

셋이 0 인 이유는 우연이 아니다 — **elaborate 의 B1 컷이 서브루틴 안의 타이밍 제어를 거부**한다(슬라이스 #3
에서 `disable fork` 두 철자로 직접 쟀다). 즉 그 세 절은 **도착할 수 없는 모양**을 적고 있었다.

⭐⭐ **남은 하나는 두 위임 실행기 중 하나만의 이야기다.** 같은 walk 가 두 종류의 바디에 적용되는데 `Call`
에 대한 답이 다르다:

| 실행기 | 도달 경로 | `Terminator::Call` |
|---|---|---|
| `run_frame_call` | 평범한 함수, `Expr::Call` | **arm 이 없다 → `break`**(중첩 호출이 조용히 건너뛰어지고 함수가 조기 반환) |
| `run_task_with` | 동기 태스크, A3-i 의 subset 경로 | **재귀**해서 formal 을 바인딩하고 output 을 복사해 돌려준다(A3-iii 가 호출자 store 를 스레드했다) |

그리고 **7 이 전부 태스크**다 ⇒ 함수에 대해서는 유지하고 태스크에 대해서만 들어 올린다. 중첩 callee 가
정지하는 경우는 이 행의 문제가 아니다 — **자기 프레임의 `frame_suspends` 행**이 잡는다.

⚠️ **유지한 절반은 도달 불가이고 그것도 쟀다** — 함수 바디가 `Terminator::Call` 을 얻는 유일한 소스
모양은 output formal 을 가진 서브루틴에 대한 중첩 호출인데, **elaborate 가 정확히 그것을 거부**하고
진단문이 이 자리를 말 그대로 지목한다(*"any position inside a FUNCTION body lowered as a call frame …
the same call in a TASK body, or in a module process, does work"*). iverilog 도 같은 소스를 거부한다.
arm 은 fail-closed 로 남기되 **IR 뮤테이션으로만 물을 수 있으므로** 합성 케이스를 지어 핀했다.

**측정**: **6,199 / 6,440 = 96.26%**(+7 · 예측 +7) · 전 스위트 **5443 green** · **flip 런**(실패 3 =
백엔드 이름 핀) · **발산 0** · **뮤테이션 4/4 사망**.

⚠️ 절차: 케이스 D 의 첫 치환이 **의미 없는 no-op**(`if false` 팔을 원본 앞에 끼워 넣어 원본이 그대로
실행)이었다 — SURVIVED 로 세지 않고 `break` 로 다시 걸었다. §5.1-s 에서 같은 실수를 한 적이 있다.

#### 5.1-ae ✅ #8 거부 시스템태스크 `$writemem*` — **행의 이유가 옳았고, 그 문장이 곧 세 읽기였다** · 96.26% → **96.35%** (2026-08-15)

`systask_refusal` 에서 **실사용 설계가 닿는 마지막 행**이었다. 이유는 *"it reads the MEMORY itself, not a
formatted argument"* — 이 파일이 §4.5.338 이래 반복해 온 *"거부 행은 자기 이유가 언제 거짓이 되는지
모른다"* 의 **반례**다: 이 이유는 참이었고, 참인 채로 **할 일의 목록**이었다. store 에 묶인 읽기는 셋:

| 읽기 | 자리 |
|---|---|
| 윈도 `start` · `finish` | `eval_task_arg` 로 스레드 — ⚠️ **A1-iii 가 `readmem` 쪽만 하고 자기 주석에 "여긴 아직 raw" 라고 적어 뒀다** |
| 원소 값 | `sched.st.read_net` → 신규 seam **`read_task_net`** |

나머지는 전부 IR/사이드카다(파일 이름 = `Const` · 타깃 넷 id = `Expr::Signal{word:None}` · `array_len`/
`width` · `declared_array_base`) ⇒ 라우팅 불필요. 미스레드면 네이티브 런이 **선언 초기값으로 가득 찬
파일**을 쓴다 — **존재하고 틀린 파일**이라 exit code 가 아무 말도 하지 않는다.

⭐ **seam 을 `queues_io` 에 둔 이유는 raw-read 핀이다** — 그 핀은 파일당 raw 읽기 수를 세는데, 두 팔짜리
match 를 호출 사이트에 인라인했으면 `crv_draw.rs` 가 **스레드된 읽기 때문에 0 이 아니게** 된다(= "아직
거부 행 뒤" 로 읽히는 숫자). 이제 `crv_draw.rs` **4 → 0**(세 번째 0 파일) · `queues_io.rs` **2 → 3**.

⭐ **도달 불가 논거를 쟀고 테스트로 핀했다.** `read_task_net` 의 `Some` 팔은 **맨 아레나**를 읽는다
(`eval_task_arg` 는 `HeapRouted` 를 지난다). 그것이 안전한 이유는 `$writemem*` 타깃이 힙·프레임 넷일 수
없기 때문이고, 그건 주장이 아니라 측정이다 — **dyn array = E3009**(iverilog 도 거부) · **whole
unpacked-array 프레임 로컬 = E3009**(⚠️ **iverilog 는 실행한다** = honest-loud 갭, §3 에 1줄). `NetArena`
의 소유권 가드가 `debug_assert!` 라 **릴리스에선 조용하므로**, 그 두 거부를 여는 슬라이스가 이 seam 을
먼저 깨뜨리도록 핀을 세웠다.

⚠️⚠️ **뮤테이션 C 가 머신을 두 번 내렸고, 그것이 제품을 바꿨다.** 내림차순 윈도에서 `step` 을 +1 로
고정하면 `loop { … if addr == finish break }` 의 **유일한 탈출 조건에 도달할 수 없다** → `body` 에 무한
append. 전 스위트 배터리로 걸린 결과: `vita` 두 프로세스가 각각 **33 GB**(32 GB 머신) → jetsam →
WindowServer 크래시 → **userspace watchdog 커널 패닉**(2026-08-14 · `panic-full-2026-08-15-055710`).
두 세션이 그렇게 죽었고 **그때마다 뮤테이션이 트리에 남았다**. ⇒ 루프를 **센티널이 아니라 카운트**로
바꿨다(`start.abs_diff(finish) + 1`) — 도달 가능한 모든 윈도에서 반복·순서·바이트가 동일하고, 바뀌는
것은 **실패 모드**다: 틀린 step 이 이제 **행이 아니라 틀린 파일**을 낸다. 재채점하니 **0.856 초에 사망**
하고, **기존 테스트 `writememh_range_inclusive_and_descending` 도 함께 잡는다** — 그 행은 원래 보호되고
있었고 뮤테이션이 행을 걸어서 채점이 안 됐을 뿐이다. 저장소에 per-test 타임아웃이 없어 `.config/
nextest.toml`(`terminate-after = 4 × 60 s`)을 신설했다(⚠️ CI 는 `cargo test` 라 이 파일을 안 읽는다).

**측정**: **6,206 / 6,441 = 96.35%**(+7 · 예측 +7) · 전 스위트 **5445 green** · **flip 런 5442/5445**
(실패 3 = 백엔드 이름 핀) · **발산 0** · differential **9 설계 3-way 0 diff**(non-zero base · 하강 선언
범위 · 넷 bound 범위밖 · x/z + 폭 6 · 태스크 formal bound · 단일 원소 · **연속대입 wire bound**) ·
뮤테이션 **4/4 사망**.

⚠️ **census 가 다음 표적 순서를 정정했다**(§5.1-p 규칙대로 매 슬라이스 착수 전 재측정): 잔여 235 를
**행이 아니라 집합**으로 세면 `real`(D+S 짝) **73** · `frame-suspends + call-suspends`(A3-ii-b, fork 행
**없이**) **42** · `probe` 5 · out-of-window write 5 · concat heap chunk 3 이고, **fork 행 106 은 그 둘과
얽혀 있다**(단독으로 닫아도 설계를 못 얻는다). ⇒ 다음은 **real 이 가장 크다.**

⚠️ 이 행에 남은 인구는 **`$dumpall`/`$dumpon` 1 설계**뿐이고 그것은 **이 슬라이스가 손으로 적은 것**이다
(코퍼스 인구 0 — 거부-행 테스트 셋을 `$writemem*` 에서 다시 철자해야 했다. 다섯 번째 재철자다).

#### 5.1-af ✅ A6 `real` — **"S2 width class" 였던 적이 없다 · 없던 것은 플래그 하나와 계수 두 팔** · 96.35% → **97.47%** (2026-08-15)

§5.1-b 가 말한 **D+S 짝**의 마지막이자 가장 큰 것(잔여 235 중 **73**). 두 행이 같은 기능을 두 번 이름
불렀다 — design 게이트의 `real`, storage 게이트의 `real: S2 width class`.

⭐⭐ **행의 이름이 틀렸다.** f64 의 64 비트는 **평범한 워드 저장**이고 엔진도 정확히 거기에 둔다
(`NetSlot::is_real` 은 플래그일 뿐, 값은 같은 비트 평면에 있다). 필요한 것은 새 저장소가 아니라 ⓐ
**플래그**(`Slot::is_real`, 모든 읽기에 스탬프)와 ⓑ **real↔int 대입 계수의 나머지 두 팔**이었다.

⭐ **거부 행 둘이 자기가 요구하는 것을 이미 적어 뒀고, 하나는 이미 지어져 있었다** — `arena.rs` 의
*"S2 OBLIGATION … must carry an `is_real` slot flag through BOTH the OOB and in-range arms"* 가 그대로 할
일 목록이었고, `wprog.rs` 의 *"Whoever lifts that arena row would otherwise silently hand `-0.0` to the
integer path"* 는 **가드가 이미 있었다**(kind 검사) ⇒ 이 슬라이스의 `wprog` 코드 **0 줄**.

⭐ **재진술이 아니라 추출.** 엔진은 2×2 네 팔을 다 갖고 tier-3 은 **하나만** 갖고 있었다(S1c 가 손으로
미러 — real VALUE 는 real NET 없이도 도달한다). `value::coerce_assign` + `value::whole_net_dest` 로 뽑아
둘 다 같은 규칙을 부르고, store 에 남는 것은 *"이 넷이 real 인가"* 와 정수 목적지의 폭·부호뿐이다.

⚠️ **`whole_net_dest` 에서 `word` 를 빼는 것이 핵심** — `real m[0:3]` 의 **원소도 real** 이라
`word.is_none()` 을 넣으면 `m[2] = m[1] + m[3]` 이 (false,true) ROUND 로 가서 6.0 이 정수 6 이 된다
(뮤테이션 F 가 정확히 그것이고 앵커가 잡는다).

⚠️⚠️ **내가 적은 이유가 틀렸고 뮤테이션이 반증했다** — OOB arm 의 스탬프를 *"`%f` 렌더를 가른다"* 고
적었는데 **안 가른다**(all-X real 도 all-X integer 도 `0.000000`). 진짜 판별자는 **`truthiness`** 다:
real 은 "0 이 아닌가", integer 는 "X 면 UNKNOWN" ⇒ `m[9] ? 1 : 0` 이 `0` 대 **`X`**(iverilog `0`).
주석은 실측대로 다시 썼다.

⚠️ **생존 1 은 등가가 아니라 도달 불가이고 이유를 쟀다** — `eval_store_word` 의 real 거절은 융합 op 를
고르는 `plain_scalar_dest` 가 엔진의 `plain_scalar`(첫 절 `!is_real`)를 보기 때문에 발화하지 않는다
(`panic!` 프로브 **0 히트**). **fail-closed 로 남겼다** — `build_plain_scalar` 한 줄이면 여기 실패가
조용해진다.

⚠️⚠️ **거부 핀 셋이 전부 `real` 을 이름으로 부르고 있었고, 그중 하나가 하네스 갭을 드러냈다** —
`native_gate.rs` 의 `sidecar_opts` 에 **`fork_modes` 가 없어서** fork 설계가 그 파일 안에서는 `eligible`
로 보였다(**열두 번째**). 셋 다 `fork`/`disable_fork` 로 재철자했고 `backend_flag` 쪽은 오히려 **더
날카로워졌다**: `real` 은 scope·storage 둘 다 false 라 "두 반쪽이 하나로 접히는 것" 을 원리적으로 못
잡는데 `fork` 는 **scope=false ∧ storage=true** 다.

⚠️ **design 게이트의 net-KIND 루프에 이제 거부 arm 이 하나도 없다** — 모든 `NetKind` 가 core 아니면
admitted 다. 루프는 `_`-free 문서로 남긴다(새 kind 는 강제 결정).

⚠️ **`$bits(real)` 은 앵커에서 뺐다** — vita·**verilator 64** / iverilog **1**(iverilog 가 outlier ·
vita 가 spec-correct). 알려진 발산이 앵커에 들어가면 앵커가 아니다.

**측정**: **6,279 / 6,442 = 97.47%**(+73 · 예측 +73) · 전 스위트 **5446 green** · **flip 런 5443/5446**
(실패 3 = 백엔드 이름 핀) · **발산 0** · differential **10 설계 3-way 0 diff**(int→real·real→int 반올림
양방향·real 산술·real 배열 원소·프레임 formal/반환·concat 목적지·비트선택 목적지·NBA+변화감지·
`$realtobits`/`$bitstoreal`/N6 math·OOB 원소) · 뮤테이션 **5/6 사망 · 생존 1 = 도달 불가(실측)**.

⚠️ **다음 표적**(같은 census, 집합 단위): **A3-ii-b(`frame-suspends`+`call-suspends` 짝) +42** →
**A4 fork +107**(단, fork 는 **단독 이득 0** — 107 이 전부 A3-ii-b 의 행을 함께 이고 있다) → 꼬리
(probe 5 · out-of-window write 5 · concat heap chunk 3 · `$dumpall` 1).

#### 5.1-ag ✅ A3-ii-b 정지하는 프레임 — **없던 것은 실행기가 아니라 스택의 수명이었다** · 97.47% → **98.09%** (2026-08-15)

두 행(storage *"a task frame that SUSPENDS"* · executor *"a call statement whose callee suspends"*)이
거부한 것은 **워크가 못 하는 모양이 아니었다** — A3-ii-a 가 이미 프레임 CFG 를 돌린다. 거부한 것은
**수명**이다: 워크가 열린 프레임을 지역 `Vec` 에 들고 있어서 프레임 안의 `Delay`/`Wait` 가
`Step::Suspended` 를 반환하는 순간 스택이 통째로 버려졌다. `body.rs` 가 그 전제를 자기 주석에 적어 뒀다 —
*"그 게이트는 `Delay`/`Wait`/`Fork` 에 닿지 않는 프레임만 admit 하므로 stash 할 것도 복원할 것도 없다."*

⭐ **착수 전 census 가 절을 갈랐다** — 정지하는 프레임의 원인은 **`wait_edge` 65 · `delay` 19 · `fork` 13**
이고 중첩은 5. fork 를 가진 설계는 **전부 `D:fork` 도 켠다** ⇒ 이 슬라이스는 앞의 둘을 열고 fork 는 A4 에
남긴다. 행이 *"정지한다"* 에서 **"fork 에 닿는다"** 로 좁아졌고, `frame_suspends` 는 `frame_park_kinds`
(timed/fork)로 갈라져 **한 walk 이 두 술어를 먹인다**.

⭐ **재진술이 아니라 추출 + 타입 통합.** 윈도 stash 는 엔진의 것이고(`stash_frame_windows`) 그 안에서
`SimState::frame_stack` 을 pop 하는 **순서가 틀리면 조용히 남의 프레임을 읽는다** ⇒
`frame_window::{stash,restore}_windows_in(st, &mut [FrameRec])` 로 뽑고 **엔진이 위임**한다. 그리고
tier-3 의 `OpenFrame` 은 A3-ii-a 가 *"park 하지 않으므로 `window`·`dyn_parked`·`forked`·`is_arm` 이 없다"*
고 정의한 **축소판**이었다 — 그 전제가 끝났으므로 **`FrameRec` 로 합쳤다**(없던 필드가 곧 할 일이었다).

⚠️ **Scheduler::activities 는 쓰지 않았다** — S0 게이트가 fork 를 거부하므로 tier-3 의 activity 는 곧
프로세스이고, 커널이 **프로세스 id 로 키잉된 맵** 하나면 된다. 엔진이 activity 아레나를 필요로 하는
이유(fork 자식)가 여기엔 없다.

⚠️⚠️ **잠복 결함 하나를 함께 고쳤다** — `wait(e)` 가 **이미 참일 때의 폴스루**가 `bb = *resume`(프로세스
철자)로 적혀 있었다. 프레임이 `Wait` 에 닿을 수 없던 동안은 무해했고, 이 슬라이스가 도달 가능하게
만들었다: 그대로면 워크가 같은 블록을 계속 다시 가져와 step guard 로 죽는다.

⚠️⚠️ **뮤테이션 첫 라운드에서 그 수정과 walk 의 전이 절이 **둘 다 생존**했다** — 앵커에 `wait(e)`-참
형태가 없었고, 코퍼스의 forking frame 은 전부 **정지 없이** fork 에 닿아 `Delay` 의 resume 엣지를 따라가는
절에 설계가 0개였다. 판별자 둘을 지어 **5/6 사망** · 생존 1(빈 스택 저장)은 **실측 등가**.

⚠️ **행이 좁아지며 핀 5개가 걸렸고 둘은 positive 로 뒤집혔다** — `a_parking_callee_is_refused_by_the_storage_layer`
→ `..._is_admitted_by_both_layers`, `a_hierarchical_enable_whose_callee_parks_is_still_refused` →
`..._now_runs_natively`(계층 enable 로 들어간 프레임이 park 하고 resume 한다 = A3-iv 가 "도달 불가" 로
남겨 둔 바로 그 축). 프레임 행-커버리지 표는 **9 → 6**: 옮겨간 셋을 대체할 fork 설계를 지을 수 없다
(DESIGN 게이트가 먼저 거부한다) ⇒ 그 사실을 기록하고 행은 fail-closed 로 남겼다.

**측정**: **6,321 / 6,444 = 98.09%**(+40 · 예측 +42) · 전 스위트 **5448 green** · **flip 런 5445/5448**
(실패 3 = 백엔드 이름 핀) · **발산 0** · 3-way probe 6종(에지 park · delay park · **동시 park 되는
automatic 프레임 둘** · 중첩 park · 계층 enable park · `wait` 참-폴스루) · 뮤테이션 **5/6 사망**.

⚠️ **다음은 A4(fork) 하나다** — 잔여 123 중 **107 이 fork 가족**(78 + 24 + 5)이고 나머지는 꼬리
(probe 5 · out-of-window write 5 · concat heap chunk 3 · `$dumpall` 1 · X 단독 2).

#### 5.1-ah ✅ A8-probe — **행의 이유는 참이었고, 그래서 값 하나만 옮기면 됐다** · 98.09% → **98.17%** (2026-08-15)

`--probe`(G2 trace.jsonl)를 거부하던 design 행. 이유는 *"이 레일은 인터프리터의 change hook 을 탄다
(`emit_probe_change` 가 `note_change` 안에서 불린다)"* 였고 **참이었다** — 슬라이스 #6 이 `stage` 와
한 주석을 공유하던 이 문장을 갈랐을 때 *"`probe` 엔 참이고 `stage` 엔 거짓"* 이라고 측정해 뒀다.

⭐ **그런데 레일의 상태는 전부 공유였다** — `probed`·`probe_prev`·`trace_lines`·`net_names` 가 모두
`SimState` 에 있다. store 에 묶인 것은 **값 하나**(`self.nets[i].cur`)뿐이고, tier-3 에는 **VCD 용
store-point 캡처가 S1d-4d-2 이래 있다** ⇒ 그 쌍둥이를 하나 더 놓고(`probe_pending`) 커널이
`drain_probe` 로 공유 emitter 에 넘기면 끝. `emit_probe_change` 는 값을 받는 `..._from` 으로 쪼개고
엔진 팔은 **읽기를 밖으로 뺀 자기 본체**라 경로가 구조적으로 불변이다.

⭐ **캡처 자리가 곧 정확성 논거다** — 한 슬롯 안의 `v = 1; v = 2; v = 1;` 이 **세 레코드**로 남아야
한다(drain 시점 재읽기면 마지막 값 하나가 세 번 나온다). 같은 값 재기록은 **레코드 0**(공유
`probe_prev` dedup). 앵커가 그 둘을 정확한 JSON 줄로 핀한다 — iverilog 오라클이 없는 vita 고유
포맷이므로 **절대 핀**이 맞다.

⚠️ 미배선의 실패 모드는 크래시가 아니다 — `trace.jsonl` 에 **t0 줄만 있고 그 뒤가 비어 있는 채 exit 0**
이다(A7 의 `coverage.json` 과 같은 모양: 있는데 틀린 G2 산출물).

⚠️ **생존 1 은 도달 불가이고 이유를 쟀다** — 캡처를 `Some(word)` 로 바꿔도 안 죽는다. `word` 는 **원소**
인덱스이고 `--probe` 는 unpacked 배열을 **CLI 에서 E0001 로 거부**하므로 probe 대상은 언제나
`elems == 1` 이다. `Some(0)` 을 유지한 이유를 주석에 적었다.

⚠️ **테스트 둘이 걸렸고 하나는 주제가 소진됐다** — `sidecar_families_reject_from_opts` 는 "여러 사이드카
family 가 **함께** 보고된다" 를 재는데, 그 짝들이 `clocking`(#1) → `file_directed`(#4) → `stage`(#6) →
`probe`(여기) 순으로 전부 core 가 되어 **`fork` 하나만 남았다**. 세는 절반(한 family 의 단위 = 테이블
엔트리 수)은 유지하고 together-ness 는 **사이드카 + 문장 스캔** 짝으로 옮겼다.

**측정**: **6,327 / 6,445 = 98.17%**(+6 · 예측 +5) · 전 스위트 **5449 green** · flip **5446/5449** ·
**발산 0** · vm/native `trace.jsonl` **바이트 동일**(글리치·dedup 포함) · 뮤테이션 **3/4 사망 · 생존 1 =
도달 불가(실측)**.

#### 5.1-ai ✅ A8-concat — **행이 자기 후속 작업을 적어 뒀고, 그것은 두 번째 철자가 아니었다** · 98.17% → **98.22%** (2026-08-15)

V1 슬라이스 2 가 **일부러 거부**한 행이다(`{d[0], x} = …` 처럼 concat lvalue 안에 힙 청크가 있는 모양).
그 행은 이유와 **후속 작업까지** 적어 뒀다 — *"라우팅하려면 소스를 청크마다 갈라 서로 다른 store 로
보내야 하는데, 그 분할 규칙은 이미 `NetArena::write_lvalue` 에 있다. 라우터에 두 번째 철자를 쓰는 것은
§4.5.279 부류의 결함이다. 올바른 지원은 **funnel 에 per-chunk escape** 를 주는 것"*.

⭐ **그대로 지었다.** `write_lvalue_escaping(… , &mut Escape)` — 분할(MSB-first, 청크마다 저정렬 조각)은
**정본 한 곳에서 한 번** 일어나고, `Escape` 는 *"이 인덱스의 조각은 내 store 것이 아니다"* 만 말한다.
어느 store 가 넷을 소유하는지는 커널만 알므로 마스크는 라우터가 만들고, `Escape::none()` 을 넘기는 기존
호출자 둘은 **경로가 구조적으로 불변**이다.

⚠️ **탈출한 조각은 아레나 청크들 *뒤에* 적용되고 그것은 관측 불가다** — 한 concat lvalue 의 청크들은
서로소 목적지이고 그 사이에 읽기가 없다(A1-iii 의 `TaskWrites::Collect` 와 같은 논거, 같은 이유:
collect 를 강제한 것이 바로 그 빌림이다).

⚠️⚠️ **iverilog 는 이 모양에서 오라클이 아니다 — 내부 assert 로 abort 한다**
(`ivl_stmt_lvals(net) == 1`, `show_stmt_assign_sig_darray`). 값은 **hand-IEEE §11.4.12**(concat lvalue 는
소스를 MSB 부터 가져간다)로 검산해 앵커에 박았다.

⚠️ **"게이트가 admit 한다" 와 "써진다" 는 다른 주장이고, assoc 청크가 그 차이다** — assoc 키는
`(offset, word)` 쌍을 못 타므로 **양 백엔드 모두 `W4020` 로 loud** 하게 무시한다(엔진의 기존 동작이지
이 슬라이스가 만든 것이 아니다). 둘을 각각 다른 테스트에 핀했다.

⚠️ **생존 1 은 실측 등가이고 이유가 유익하다** — escape 한 청크를 아레나가 *함께* 써도 오늘은 안 잡힌다:
힙 넷의 아레나 슬롯은 **죽어 있고** `write_chunk` 는 `assert_owns` 를 부르지 않으며 힙 넷의 dirty·VCD 를
소비하는 것이 없다. 원리적으로는 무해하지 않으므로 `continue` 는 이유와 함께 남겼다. ⚠️ 케이스 하나는
**BUILD-FAIL 이었고 SURVIVED 로 세지 않았다**(§5.1-l), 하나는 **철자 불가**다(조각이 아레나 호출에서
나오므로 "그 앞에 적용" 하는 순서가 존재하지 않는다).

**측정**: **6,332 / 6,447 = 98.22%**(+5 · 예측 +3) · 전 스위트 **5451 green** · flip **5448/5451** ·
**발산 0** · vm/native 값 동일 · 뮤테이션 **2 사망 · 1 등가(실측) · 1 철자 불가**.

#### 5.1-aj ✅ A3-iii-b out-of-window write — **21/23 이 공유 저장소였고, 좁히기만 했으면 조용히 틀렸다** · 98.22% → **98.31%** (2026-08-15)

A3-iii 가 행을 *"names"* 에서 *"WRITES"* 로 좁히며 이유를 적어 뒀다 — *"위임 바디의 모든 목적지는
`SimState::frame_write_lvalue` 를 지나는데 그것은 `&self` 라 호출자의 아레나에 닿지 못한다"*.

⭐ **그 이유는 flat 목적지에 대해 참이고**(그 함수는 목적지가 프레임 슬롯임을 `debug_assert` 한다 —
**엔진조차 못 한다**) **클래스 필드에 대해서는 거짓이다**: 그 쓰기는 `class_heap` 으로 가고 그것은 두
커널이 같은 것을 빌린다. 계측: 스위트가 닿는 창 밖 쓰기 **23 사이트 중 21 이 클래스 필드 · 4 가 flat**.

⚠️⚠️ **그런데 행을 좁히는 것만으로는 silent-wrong 이었다 — 첫 probe 가 잡았다.** 힙 store 는 라우팅이
필요 없지만 **그 store 를 키잉하는 객체 id 가 넷에 산다**: `frame_or_class_write` 가 그 핸들을 `self`
에서 읽어 네이티브 런에서는 t0 값 0 = null 을 보고 **필드 쓰기를 그럴듯한 경고와 함께 버렸다**
(`v=0 w=0` vs iverilog `v=6 w=7`). A2-i 가 읽기 쪽에서, A2-ii 가 CRV 수신자에서 잰 바로 그 모양이다.

⚠️⚠️ **그리고 그 수정의 첫 형태도 틀렸다 — 값 테스트 둘이 잡았다.** 호출자 store 를 **맨손으로** 넘기면
핸들이 **프레임 슬롯**일 때(메서드의 `this`) 아레나에 없어서 또 null 이 된다. 정답은 읽기 쪽이 쓰는
**복합 리더 `HeapRouted`** 다 — *"라우팅은 한 군데가 아니다"* 의 다섯 번째.

⚠️ **좁힌 뒤 이 행의 도달 가능한 인구는 0 이다**(census 로 확인). flat 창 밖 쓰기는 elaborate 가
거부하고(**세 철자 실측**: 직접·지역변수 경유·`c = new()`) 비-`automatic` 태스크는 인라인돼 프레임이
없다. 행은 fail-closed 로 남겼다.

⚠️⚠️ **하네스 갭 열세 번째** — `native_gate.rs` 의 `sidecar_opts` 에 **`class_handle_nets` 가 없어서**
그 파일 안에서는 클래스 필드 쓰기가 여전히 flat 으로 보였고, **테스트가 이미 거짓이 된 이유로 통과**
하고 있었다. 심자마자 드러났고, 그 테스트의 storage arm 은 이제 **손상 사이드카**로 재철자했다(소스로
만들 수 있는 eligible ∧ !buildable 설계가 더는 없다).

⚠️ 거부 핀 **6개**를 처리했다 — 셋은 positive 로 반전(이름이 거짓이 된 둘은 **개명**), 행-커버리지 표는
**6 → 2**(클래스 필드 설계 넷 제거), CLI 의 storage arm 은 반전, 스캔 테스트는 손상 사이드카로.

**측정**: **6,338 / 6,447 = 98.31%**(+6 · 예측 +5) · 전 스위트 **5452 green** · flip **5449/5452** ·
**발산 0** · 뮤테이션 **3/4 사망 · 생존 1 = 도달 불가(실측)**.

#### 5.1-ak ✅ A5-dumpall — **거부 집합이 비었다** · 98.31% → **98.29%**(모수 +2) (2026-08-15)

`$dumpall`/`$dumpon` 은 `systask_refusal` 의 **마지막 두 멤버**였고, 그 행은 정확하고 **함수 하나만큼**
넓었다 — *"둘은 `full_snapshot` 을 통해 재스냅샷하는데 아레나 리더를 스레드하는 것은 `$dumpvars` 호출
사이트뿐이다"*. 나머지 두 호출 사이트를 스레드한 것이 이 슬라이스의 전부다(`dump_on_with` /
`dump_all_with`). 그 외의 모든 것 — `dumping`·id 테이블·writer — 은 이미 `SimState` 것이다.

⭐⭐ **`systask_refusal` 의 집합이 이제 비었다**(6 → 4 → 2 → 0). 함수와 **두 소비자는 남긴다** —
`k_dispatch_systask` 의 panic 과 런 게이트의 행 — 새 `SysTaskId` 가 스레드 안 된 store 를 읽으면 조용한
오답이 아니라 거부가 되게 하는 것이 그 둘이고, 오늘 핀하는 것은 *"지금 비어 있다"* 다.

⭐ **미배선의 실패 모드는 크래시가 아니다** — 엔진 store 를 다시 읽으므로 `$dumpon`/`$dumpall` 이
**선언 초기값의 스냅샷**을 파형에 적는다(다른 레코드는 맞은 채로, exit 0).

⭐ **앵커의 판별자는 "그 값이 파일의 다른 곳에 없다"** 는 것 — `a` 를 **덤프가 꺼져 있는 동안** 바꾸므로
`$dumpon` 이 싣는 값은 변화 스트림에 존재하지 않고, `b` 는 t0 이후 안 변하므로 **세 스냅샷에만** 나온다
(스냅샷이 변화 재생이 아니라 전 설계 워크임을 보이는 줄). ⚠️ 핀은 vita 자신의 VCD 텍스트다 — iverilog 는
선행 0 을 떼고(`b100010`) 파라미터용 블록을 앞에 붙이며 끝에 `#6` 을 적는다(전부 기존 포맷 차이) ⇒
**의미만 iverilog 로 확인**(off ⇒ all-X · on/all ⇒ 현재값)하고 두 백엔드는 **바이트 동일**.

⚠️ **호출자 없는 래퍼 셋을 지웠다**(`dump_on`/`dump_all`/`full_snapshot`) — 두 사이트가 다 스레드되면
`None` 래퍼는 호출자가 없고, **호출자 없는 seam 은 짝 없는 행과 같다**(A1-iv-a 규칙).

⚠️ **테스트 셋의 주제가 소진됐다** — 거부 개수 핀은 **2 → 0**, 실행기 행-커버리지 표는 **3 → 1**(두
`$dumpall` 설계 제거), CLI 폴백 핀은 **일곱 번째 재철자**로 **bare `wait fork`** 가 됐다(그것이 eligible
을 유지한 채 실행기 층에 닿는 유일한 모양 — `fork_modes` 를 안 만들어 S0 fork 행에 안 보인다).

**측정**: **6,339 / 6,449 = 98.29%** — ⚠️ **분자 +1, 분모 +2**(이 슬라이스가 추가한 앵커와 **일부러
거부되는** `wait fork` 핀 설계) 때문에 비율은 내려갔다. 전 스위트 **5453 green** · flip **5450/5453** ·
**발산 0** · vm/native VCD **바이트 동일** · 뮤테이션 **3/3 사망**.

⚠️ **잔여 110 은 전부 fork 가족이다**(78 + 24 + 5 + `wait fork` 3) — Phase A 의 꼬리가 끝났고 남은 것은
**A4 하나**다.

#### 5.1-al ✅ A4 착수 — **fork 부기를 큐에서 떼어냈다**(전제조건) · 커버리지 불변 (2026-08-15)

> 이어받은 것은 **§5.1-am**(A4-a) 이고 아래 5단계 표를 그대로 실행했다. 남은 A4 =
> fork-in-frame 24 · `wait fork`/callee-forks 7 · `disable fork` 6.


Phase A 에 남은 유일한 슬라이스이고 **잔여 110 = 100% fork 가족**이다(fork 78 · frame-forks 24 ·
disable_fork 5 · `wait fork` 3). 착수 census 가 이미 정한 사실: **arm 이 전부 직선인 설계는 13%(14/108)
뿐**이고 87% 는 실제로 park 하므로, "직선 arm 만 인라인" 같은 부분 구현은 **진짜 슬라이스가 다시
걷어내야 하는 버리는 작업**이다 ⇒ 짓지 않는다.

⭐ **설계 = 재구현이 아니라 위임.** 엔진의 fork 기계장치는 전부 `Scheduler` 에 있고 `NativeKernel` 은
이미 `&mut Scheduler` 를 든다. 재사용 못 할 유일한 조각은 **큐 push** 다(엔진은 `self.cur.active`, tier-3
은 `native::wake`). 그래서 이 단계가 한 것: `exec_fork` 와 `on_child_complete` 에서 **push 만 떼어내
`_into(…, ready: &mut Vec<Ready>)` 로** 만들고 엔진은 이전 본체를 그대로 감싸 호출한다(구조적 불변).

⚠️ **떼어낸 것이 push 뿐인 이유가 곧 정확성 논거다** — barrier 등록·tie 합성과 그 오버플로 가드·
fork-in-frame 의 윈도 공유·`JoinMode` 결정·`wait fork` 카운트다운은 전부 큐 독립이고, **두 철자가 되면
조용히 틀린다**: under-decrement 는 All-barrier 를 **조기 발화**시킨다(크래시가 아니라 오답). tie 순서도
같은 부류다 — 형제 arm 의 결정성이 거기 걸려 있다.

**남은 A4 의 단계**(다음 슬라이스가 이어받는다):

| # | 무엇 | 크기 |
|---|---|---|
| 1 | tier-3 의 `NativeReady{proc,…}` 를 **activity id** 로(기본 활동은 1:1 이라 값은 같다) | ~30 사이트, 기계적 |
| 2 | tier-3 이 `Scheduler::activities` 를 **실제로 시드**(오늘 `arm_t0` 는 안 만든다) | 작음 |
| 3 | 워크의 `Terminator::Fork` arm → `k_exec_fork` seam → `exec_fork_into` | 작음 |
| 4 | 자식 완료(`Step::Done` ∧ `join_ref`) → `on_child_complete_into` → tier-3 큐에 부모 재적재 | 작음 |
| 5 | 게이트 행 넷(`fork`·`disable_fork`·frame-FORKS·실행기 행의 fork 절) | 작음 |

⚠️ 순서 주의: **`join_none` 은 인라인 실행과 관측이 다르다**(부모가 먼저 계속되고 자식은 같은 델타에서
뒤에 돈다) — 그래서 arm 을 순차 인라인하는 지름길이 `join` 에만 성립하고, 그 지름길을 안 쓰는 이유다.

#### 5.1-am ✅ A4-a 프로세스 레벨 fork — **재구현이 아니라 위임, 그리고 없던 것은 자식이 어디서 끝나는지였다** · 98.29% → **99.43%** (2026-08-15)

**+76 설계.** `D:fork` 행이 **107 → 0** 이다. 코퍼스 6,452 중 **거부가 37 개**만 남았고 그 37 은 셋으로
정확히 갈린다 — **fork-in-frame 24**(`S:` + `X:` 동시) · **`wait fork`/callee-forks 7**(`X:` 단독) ·
**`disable fork` 6**(5 단독 + 1). 즉 A4-a 는 fork 가족의 **프로세스 레벨 절반**을 통째로 가져왔고,
남은 셋이 A4-b/-c/-d 다.

⭐ **§5.1-al 이 적어 둔 5단계가 그대로 맞았다** — 활동 id 도입(`NativeReady.proc` = activity) ·
`arm_t0` 가 `seed_base_activities()` 를 부름 · `Terminator::Fork` arm → `k_exec_fork` →
`exec_fork_into` · 자식 완료 → `on_child_complete_into` · 게이트 행 삭제. 재구현은 **한 줄도 없다**:
barrier 등록·tie 합성·윈도 공유·`JoinMode` 결정이 전부 `Scheduler` 것이고, 커널이 대는 것은 **자식이
올라갈 큐** 하나다(`push_sorted_native` 를 `&mut self.active` 에 — 엔진의 `cur.active` 와 같은 자리라
arm 이 자기를 fork 한 배치 중간이 아니라 **같은 순간의 다음 델타**에 돈다).

⭐⭐ **없던 것은 fork 기계장치가 아니라 "자식이 어디서 끝나는가" 였다.** 첫 판이 정확히 그것을 빠뜨렸고
증상은 크래시가 아니라 **exit 0 의 틀린 순서**였다 — `fork a=1; b=2; join` 이 `a=1 b=0 c=1` 을 찍었다
(첫 arm 이 join 블록을 **뚫고 부모의 continuation 을 실행**한 뒤 둘째 arm 이 시작됐다). 수정은
`run_process` 의 것과 **같은 자리·같은 이유**: 종료 판정을 터미네이터에 걸면 안 된다(arm 은 `Goto` 로도
`Branch` 로도 `Delay`/`Wait` 재개로도 join 에 닿는다) ⇒ **블록 fetch 직전**에 한 번, `join_bb` 는 실행되지
않는 센티넬이므로 **거기 도착한 것이 곧 arm 의 끝**이다. `frames.is_empty()` 로 게이트하는 것도 엔진과
같다 — in-frame 자식의 `bb` 와 센티넬은 **다른 블록 공간**이라 넘어서 비교하면 숫자 충돌로 자식을 죽인다.

⭐ **`tie` 필드가 돌아왔고, 그 필드의 doc 이 자기가 돌아올 이유를 이미 적어 뒀다** — *"`fork_modes` 가
비어 있지 않은 것은 S0 거부라 여기선 `tie == proc` 이고 이 필드는 같은 수의 두 번째 이름"*. fork 가
그것을 끝낸다: 자식의 tie 는 부모 것과 arm 인덱스로 **합성**되고, 형제를 정렬하는 것은 활동 id 가 아니라
tie 다(활동 id 는 `free_activities` 프리리스트에서 나오므로 **할당 순서 ≠ 선언 순서**). 기본 활동은
여전히 `tie == proc` 이라 **기존 설계의 순서는 불변**이다.

⚠️ **`busy` 는 기본 활동만의 것이다** — `r.proc == tmpl` 가드 없이는 자식 활동 id 로
`wake.busy[…]` 를 인덱싱해 **범위 밖 패닉**이 난다(실측). `busy` 가 억제하려는 것은 정적 감도 wake 이고
그것은 프로세스의 성질이지 활동의 성질이 아니다.

⚠️ **`act_template` 의 doc 이 이 슬라이스가 방금 거짓으로 만든 문장을 들고 있었다** — *"오늘 모든 tier-3
활동은 자기 프로세스라 이것은 항등"*. §4.5.338 의 그 부류이고, 이번엔 **그 문장이 자기가 무엇을 기다리는지
까지 적어 뒀다**(*"`Fork` arm 이 서른 사이트가 아니라 이 한 함수를 바꾸도록"*) — 예측이 맞았다.

⭐ **앵커는 iverilog 절대 핀 셋이고 각각이 다른 것을 판별한다**(`cli/tests/fork_join.rs`) —
`join`+park 하는 arm(순서가 선언이 아니라 스케줄러가 정한다 · 위의 silent-wrong 을 잡는 행) ·
`join_any`(재개 시점의 `a=0` 이 "첫 arm 에 깨어났다" 를 · 뒤의 `late a=1` 이 "잉여 arm 이 안 죽었다" 를) ·
`join_none`(부모가 **먼저** 계속된다 = arm 을 선언 순서로 인라인하는 지름길을 원리적으로 배제하는 유일한
모드). 셋 다 `run.json` 의 `"backend": "native"` 를 함께 단언한다.

⚠️ **행이 열리자 그 행을 이름으로 부르던 핀 넷이 걸렸고 하나는 주제가 소진됐다** —
`sidecar_families_reject_from_opts` 는 `fork` 로 거부를 재고 있었는데 그 행이 없어져
`disable_fork` 로 재철자했고, 그 과정에서 **하네스 갭 열두 번째**를 쟀다(`native_gate.rs` 의
`sidecar_opts` 에 `fork_modes` 가 없어 fork 설계가 거기선 적격으로 보였다).

**뮤테이션 7 · 사망 5 · 생존 2 는 등가이고 둘 다 쟀다.**

⚠️⚠️ **배터리가 첫 패스에서 셋을 SURVIVED 로 잘못 보고했고 원인은 배터리의 필터였다** —
타깃을 `-p sim-engine -p cli --test fork_join …` 로 적었는데 **cargo 는 `--test` 를 선택된 모든
패키지에 건다** ⇒ sim-engine 유닛 테스트가 통째로 빠져 **5457 이 아니라 58 개**가 돌았다.
§5.1-aa 가 적은 그 실패의 재발이고, 이번엔 **ENGINEERING_RULES 의 규칙 자체가 원인**이었다
(*"배터리는 `-p sim-engine` 으로 스코프하라 — tier-3 의 킬러는 전부 거기 산다"*). ⇒ 그 규칙을
정정했다: **비용은 빌드가 지배하므로 필터가 아끼는 것은 케이스당 ~30초뿐이고, 대신 가짜
SURVIVED 를 산다** ⇒ `--workspace`.

⚠️⚠️ **그리고 그 정정이 필요했던 이유가 이 슬라이스의 실측으로 증명된다 — 다섯 케이스의 킬러가
전부 `cli::fork_join` 이고 5457 중 다른 어떤 테스트도 하나도 안 잡았다.** `-p sim-engine` 이었으면
**7/7 SURVIVED**. §5.1-e(오라클 부식)의 가장 선명한 형태다 — 게이트가 fork 를 막고 있었으므로
**fork 를 네이티브로 도는 테스트가 저장소에 0개**였다.

⚠️ **생존 B 는 등가가 아니라 앵커의 눈먼 축이었다** — 형제 정렬 키를 `tie` 에서 **활동 id** 로
바꾸는 뮤테이션이 앵커 셋을 전부 통과했다. **한 번 fork 하는 설계로는 원리적으로 못 잰다**(자식이
선언 순서로 할당돼 두 키가 우연히 일치한다) ⇒ 판별자는 **두 번째 fork** 다: 끝난 자식의 슬롯이
`free_activities`(**LIFO**)로 돌아가므로 둘째 fork 의 arm 은 **첫 fork 의 arm 이 끝난 순서**가 정한
id 를 받는다. 실측 — 뮤테이션을 걸자 `B1 B2 B3` 이 **`B2 B1 B3`** 이 됐다(exit 0 의 틀린 순서).
⚠️ **이 행의 오라클은 iverilog 가 아니다** — IEEE 는 동시 arm 의 순서를 규정하지 않고 iverilog 13 은
zero-delay arm 을 **역순**(`B3 B2 B1`)으로 돈다 ⇒ 앵커는 **절대값 + 세 백엔드 일치**로 지었다.

⚠️ **생존 F·G 는 등가이고 `panic!` 프로브로 쟀다**(전 스위트 5457, 히트 0) — **F**(자식이 컴파일
바디를 타는 것)는 `is_codegen_able` 의 `_`-free match 가 구조적으로 막는다: `act != tmpl` 을 만들 수
있는 터미네이터는 `Fork` 와 `Call` 둘뿐인데 **둘 다 그 거부 집합에 있다** ⇒ 자식을 만들 수 있는
바디는 애초에 컴파일되지 않는다. **G**(`is_child` 조기 반환)는 `Activity` 생성 사이트가 **정확히
둘**이고 둘이 `is_child`/`join_ref` 를 한 쌍으로 세팅하므로 `a.join_ref?` 가 이미 같은 답을 낸다.
둘 다 **fail-closed 반쪽으로 남기고 측정을 코드에 적었다**.

⭐ **앵커 다섯 번째는 배터리가 아니라 축 검토가 찾았다** — arm 이 **park 하는 태스크를 호출**하는
형태가 `frames.is_empty()` 가드를 **거짓 방향으로 실행하는 유일한 모양**이고(나머지 넷에선 공허하게
참이다), 동시에 A4-a 를 A3-ii-b 위에 쌓는다(arm 마다 프레임 창이 따로 park/restore 돼야 `a=20`·
`b=10` 이 갈린다 — 창이 새면 둘이 같은 수를 읽는다). 3-way 일치. ⚠️ **중첩 fork 는 범위 밖**이고
양 백엔드가 똑같이 **E3009 loud** 다(실측).

전 스위트 **5458 green** · flip 5453/5456 · **발산 0**(실패 3 은 전부 *"기본 백엔드가 vm"* 핀).

#### 5.1-an ✅ A4-b fork-in-frame — **행 하나가 아니라 셋이었고, 셋째는 첫 프로브가 찾았다** · 99.43% → **99.80%** (2026-08-16)

**+33 설계**(예측 +24 · 차이는 이 슬라이스가 더한 앵커와 복원된 테스트가 분모에 들어간 것). 코퍼스
6,461 중 **거부는 13 개**이고 전부 fork 가족의 마지막 둘이다 — **A4-c `wait fork`/callee-forks 7** ·
**A4-d `disable fork` 6**.

⭐⭐ **닫아야 할 것은 행이 아니라 셋이었다.** 착수 census 는 24 설계가 `S:a task frame that FORKS` 와
실행기 행에 **항상 함께** 걸린다고 말했고, 이유는 **둘이 같은 술어(`frame_call::frame_forks`)를 두 군데서
읽기** 때문이다. 그 짝을 빼자 **세 번째 행이 드러났다** — `contains_shared_fork` 이고,
`frames_admitted` 가 **첫 `Err` 만 돌려주므로** census 조차 그것을 못 봤다. 발견은 추론이 아니라
**첫 Case-B 프로브**였다: 짝을 지운 뒤에도 `buildable: false`, 메시지는 *"a subroutine with a shared fork
window"*. ⭐ **그리고 그 행은 A3-ii-b 가 일부러 남긴 백스톱이고 자기 doc 에 이유를 적어 뒀다** —
*"`frame_suspends` 가 한 행 위에서 park 로 보고하므로 dead 이지만, 그게 참이 아니게 되면 설계가 조용해지는
대신 이 행이 거부하도록 남긴다."* 정확히 그렇게 동작했다. ⚠️ **14/24 가 Case B 이므로 짝만 닫았으면
10 개만 배송됐다.**

⭐⭐ **없던 것은 fork 기계장치가 아니라 tier-3 의 프레임이 사는 자리였다.** `exec_fork_into` 는
§4.5.214 이래 in-frame 을 전부 한다 — `parent_in_frame`(콜스택에서) · `FRAME_FORK_KEY` 모드 조회 ·
`arm_callee`(arm 블록이 사는 CFG) · Case A(빈 `Owned`) / Case B(`Shared(h)` + refcount) 분기 · 그리고
arm 의 `FrameRec` 를 **자식의 `call_stack` 에 직접 쓴다**. 그런데 A3-ii-b 는 tier-3 의 파킹 프레임을
**커널 소유 `BTreeMap`** 에 뒀고 그 이유를 이렇게 적었다: *"엔진의 쌍둥이는
`activities[pi].call_stack` 이고, 이것이 아레나가 아니라 커널의 맵인 이유는 S0 게이트다 — fork 가
거부되므로 tier-3 활동은 곧 자기 프로세스이고 아레나가 구분할 것이 없다."* A4-a 가 그 절을 끝냈고,
**A4-b 에서 그 두 번째 저장소는 잉여가 아니라 틀린 것이 된다**: 스케줄러가 못 보는 자리에 있으면
fork-in-frame 이 **최상위 fork 처럼 spawn** 된다(윈도 없는 arm · 부모의 automatic 로컬을 아무도 공유
안 함). ⇒ `k_park_frames`/`k_take_frames` 가 **아레나 자체**를 쓰게 했고 `parked_frames` 를 지웠다.
윈도 반쪽은 이미 한 철자였다(`frame_window::{stash,restore}_windows_in`).

⭐ **워크에 더한 것은 셋뿐이고 전부 엔진의 것을 그 순서대로다** — ⓐ `Fork` 의 in-frame 프롤로그
(`forked = true` **먼저**, 그 다음 park = stash) · ⓑ `Some(rb)`(= `join_none`·자식 0)이면 스택을 되받고
**프레임의 PC** 를 세팅(엔진이 *"방금 stash 한 윈도를 다음 반복이 복원하는 no-op 왕복"* 이라 부르는 그것)
· ⓒ **in-frame 자식 완료 인터셉트**. ⚠️ ⓒ의 두 가드가 둘 다 load-bearing 이다 — `len() == 1` 은
**arm 이 부른 태스크의 프레임**이 같은 값의 블록 id 로 오발화하는 것을 막고(24 중 11 이 그 모양),
`is_arm` 은 비교를 **의미 있게** 만든다(최상위 fork 자식이 suspendable 태스크를 부르면 그것도 프레임이
하나인데, 거기선 `bb` 가 전역이고 `join_bb` 는 프로세스-로컬이라 숫자 충돌로 자식이 죽는다).

⚠️ **죽은 기계장치 삭제** — `frame_park_kinds`/`ParkKinds`/`frame_forks` 가 전부 write-only 가 됐다
(A3-ii-b 가 `timed` 를 열고 A4-b 가 `fork` 를 열었으므로 아무도 안 묻는다). 그 walk 의 fail-closed
early-return 둘(범위 밖 블록 id · 미해결 nested `Call` 타깃)은 **`driven_body_is_runnable` 이 같은
조건에서 이미 갖고 있다**(`fd.is_task && susp`) — 지우기 전에 확인했다. ⚠️ 그 walk 의 `Fork` arm 은
**children 과 `resume_bb` 를 따라가야** 한다(arm 블록은 이 태스크 CFG 의 일부이고 같은 윈도로 돈다).

⚠️ **거부 핀 둘을 positive 로 뒤집었다** — `the_park_walk_sees_a_fork_behind_a_delay` 는 주제가
사라져 `a_fork_behind_a_delay_inside_a_frame_is_admitted` 가 됐고,
`a_parking_callee_is_admitted_by_both_layers` 의 꼬리(*"FORK 절은 아직 저장소 층에서 거부한다 · 설계로는
못 물으니 술어로 묻는다"*)는 **실제 설계로** 물을 수 있게 되어 그렇게 바꿨다.

⚠️⚠️ **절차 사고 — 앵커를 쓰다가 기존 테스트 파일을 덮어썼다.** `crates/cli/tests/fork_in_frame.rs`
는 §4.5.214 의 24-테스트 스위트인데 새 파일이라 가정하고 `Write` 했다(514줄 → 177줄). 증상은
**측정의 테스트 수가 5458 → 5437 로 준 것**뿐이었고 나머지는 전부 초록이었다. 복원하고 앵커를
`fork_in_frame_native.rs` 로 옮긴 뒤 **flip 측정을 다시 돌렸다**(위 숫자가 그것이다). ⭐ 복원이
중요한 이유는 별개다 — 그 24 테스트는 **기본 백엔드로** fork-in-frame 설계를 돌리므로 flip 아래에서
**네이티브로** 돌고, 이 슬라이스의 가장 강한 이빨이다.

⚠️ **iverilog 는 반쪽 오라클이다** — Case A·Case B 는 iverilog 13 이 답하지만 frame 안 `join_none` 은
**내부 assert 로 abort** 한다(`of_JOIN_DETACH`, vthread.cc:3793 · 기존 `fork_in_frame.rs` 가 같은 크래시를
이미 기록해 뒀다) ⇒ 그 행은 **hand-IEEE §9.3.1/§9.3.2 + 세 백엔드 일치**로 지었다.

**뮤테이션 7 · 사망 3 · 생존 4 는 전부 이유가 다르고 각각 쟀다.**

⚠️ **생존 E 는 눈먼 축이었고 소비자의 doc 이 판별 설계를 이미 적어 뒀다.** `forked = true` 를 지우면
`park_dyn_in` 이 부모의 **프레임 로컬 dyn 배열**을 fork 시점에 힙에서 빼가므로 arm 이 X 를 읽는다 —
그 함수의 doc 이 *"a `fork begin … a[0] … end join` inside a task printed `a0=x`"* 라고 엔진 쪽에서 이미
측정해 뒀다. 앵커에 dyn 배열이 없어서 아무도 안 잡았다 ⇒ 지었고(**`arm a0=x a1=x` + `W4020`**, 그런데
**바로 다음 줄 `after a0=11` 은 맞는다** — 부모가 재개하며 배열을 돌려받는다) 즉시 사망.

⚠️ **생존 C 는 등가이고 이유가 인덱스 하나다.** 깊이 가드(`frames.len() == 1`)를 빼도 전 스위트가
통과한다 — **엔진의 쌍둥이는 load-bearing 인데**(`run_process` 는 `call_stack.last()` 를 읽으므로 가드가
없으면 더 깊은 **callee** 프레임의 `bb`(전역 id, `join_bb` 와 같은 공간)를 비교해 충돌로 오발화한다)
**이 워크는 `frames[0]` 을 읽고 그것은 항상 arm 이며, callee 가 도는 동안 arm 의 PC 는 join 이 아니라
호출 자리에 있다.** fail-closed 로 남기고 그 이유(= 이 함수의 인덱스 선택)를 코드에 적었다.
⭐ 반면 **`is_arm` 가드는 실제로 load-bearing 이고 킬러가 하나뿐**이다 —
`fork_join::a_fork_arm_may_call_a_parking_task`(A4-a 의 다섯 번째 앵커).

⚠️ **생존 F 는 값이 아니라 누수다 — 프로브로 쟀다.** `k_exit_arm_frame` 을 지워도 출력이 안 변하는데,
호출은 **도달한다**(arm 마다 한 번 · `func_has_auto` 참 · `frame_stack.len() == 1` 에서 자기 윈도를
pop). 빼면 **arm 당 stale 스택 항목 하나 + 해제 안 된 아레나 윈도 하나**가 남지만, 부모가 자기 윈도를
그 **위에** 복원하고 모든 `frame_slot_read` 가 top 을 읽으므로(`Shared` arm 은 아예 핸들로 읽는다)
오늘의 코퍼스로는 관측 불가다.

⚠️ **생존 G 도 등가이고 이유가 A5-dumpall 이다.** 게이트 walk 의 `Fork` arm 이 children/`resume_bb` 를
안 따라가도 통과한다 — 그 traversal 이 닿을 수 있는 거부 둘이 **둘 다 오늘 도달 불가**이기 때문이다
(`systask_refusal` 집합이 A5-dumpall 이래 비었고, 프레임 로컬 NBA 는 elaborate E3009 · iverilog 도
IEEE §10.4.2 를 인용하며 거부). **관측 가능한 형태가 아니라 옳은 형태로** 써 두었다.

전 스위트 **5462 green** · flip 발산 **0**(실패 3 은 전부 *"기본 백엔드가 vm"* 핀).

#### 5.1-ao ✅ A4-c `wait fork` — **행의 이유가 옳았고 그래서 그것은 waiter 가 아니었다** · 99.80% → **99.89%** (2026-08-16)

**+10 설계.** ⭐⭐ **실행기 게이트 층이 census 에서 통째로 사라졌다** — 남은 거부는 **7 개이고 전부
`D:disable_fork`** = A4-d 하나뿐이다.

⭐ **착수 census 가 행의 세 절을 갈랐고 답이 하나였다** — 행은 *"a `wait fork`, a `fork`, or a call
statement whose callee forks"* 인데 계측하니 **8 개 전부가 bare `wait fork;`** 이고 나머지 둘
(`WaitCause::Named` · 사이드카 없는 `Terminator::Call`)은 **코퍼스 인구 0** 이다.

⭐⭐ **거부 이유는 참이었고, 참인 채로 설계를 지시했다.** 행은 *"nothing in `fire_waiters` can ever
satisfy the cause, so admitting it would park the process forever"* 라고 적었다 — **맞다. `wait fork` 는
waiter 가 아니기 때문이다.** 걸어 둘 넷이 없고, 그것을 깨우는 것은 **자식 완료 부기**다
(`exec_wait_fork` 가 살아 있는 자식을 세어 park 하고 `on_child_complete_into` 가 카운트다운한다).
그리고 그 부기는 A4-a 이래 tier-3 에 있다 ⇒ 워크는 **아무도 못 발화시키는 waiter 를 거는 대신 위임
호출로 답한다**. **재개 경로는 코드가 0줄이다** — `on_child_complete_into` 가 부모를 join barrier 와
**같은 `ready` 벡터**에 넣고 `k_body_done` 이 이미 그것을 드레인한다.

**지은 것 = seam 하나(`k_exec_wait_fork`) + arm 하나.** `Scheduler` 도 오버라이드한다(바디 차분이
`K = Scheduler` 로 같은 워크를 돌릴 수 있도록 · `k_exec_fork` 와 같은 이유).

⚠️ **프레임 안 `wait fork` 는 여전히 거부이고 게이트 행이 아니라 FATAL 이다** — 엔진의
`run_process` arm 이 그렇게 한다(`mark_fatal` + `Step::Fatal` · Phase-4 follow-on). 게이트로 거부하면
tier-3 이 VM 으로 떨어진 뒤 거기서 fatal 나는데, 그것은 **다른 모든 것이 같을 때만** 같은 답이다.

⚠️⚠️ **표가 비었다 — 두 번째로.** `s1d4c2c_each_refusal_row_has_a_design` 의 케이스 수가
4 → 5 → 4 → 3 → 1 → **0** 이 됐다(A5-dumpall 이 시스템태스크 둘을, 이것이 마지막 하나를 뺐다).
§5.1-ab 선례대로 **지우지 않고 `is_empty()` 단언으로 남기고** 남은 세 절이 왜 소스에서 도달 불가인지를
적었다. ⚠️ **CLI 폴백 핀은 여덟 번째 재철자**이고 이번엔 **종류가 바뀐다** — 남은 유일한 거부가
design 게이트의 `disable fork` 라 `eligible: false` 이다(앞의 일곱은 전부 *"scope 는 통과, 실행기가
거부"* 였다) · **A4-d 가 그것을 배선하면 아홉 번째는 없다**(게이트에 거부할 것이 남지 않는다) ⇒ 그
테스트를 **positive claim 으로 바꾸라고** 적어 뒀다.

⭐ **앵커 셋의 각 줄이 다른 것을 판별한다**(iverilog 핀) — `join_none` 뒤 `wait fork`(`after fork a=0
b=0 t=0` = 부모가 안 막혔다 · `after wait … t=4` = **마지막** 자식을 기다렸다) · **자식 없는
`wait fork` 는 같은 시각에 통과**(틀리면 hang 이다 — 깨울 자식이 없다) · **`join_any` 잉여 arm**
(그 barrier 는 이미 발화했으므로 *"내 최근 barrier 가 아직인가"* 로 구현하면 그냥 통과한다 =
카운트가 **누적**이어야 하는 이유이고 곧 이 seam 이 위임인 이유).

**뮤테이션 7 · 앵커가 죽인 것 2 · 손으로 확인한 kill 1 · 등가 4 — 각각 쟀다.**

⚠️⚠️ **배터리가 진짜 결함 하나를 SURVIVED 로 보고했고, 원인은 결과 파서였다.** 케이스 D(`k_exec_wait_fork` 가 `resume_bb` 대신 0 을 넘긴다)는 부모를 **프로세스 엔트리**에서 재개시키므로 fork 를
다시 돌린다 — 손으로 걸자 출력이 t=2672 까지 끝없이 늘었다. **nextest 는 그것을 `TIMEOUT` 으로
보고하는데 러너는 `FAIL` 로 시작하는 줄만 킬러로 세고 있었다.** §5.1-aa 의 두 번째 형태(그때는 테스트
필터, 이번엔 결과 파서)이고 `ENGINEERING_RULES` 에 병합했다. ⚠️ **그리고 hang 을 만드는 케이스는
배터리에서 빼고 손으로 한 번만 재는 것이 맞다** — 4분 동안 자식이 파이프로 계속 찍는다.

⚠️ **등가 넷의 이유가 전부 다르다.** **C**(`k_suspend_on` 을 추가로 건다)는 `fire_waiters` 의
`_ => false` 가 `Fork` 원인을 절대 발화 안 시켜 **엔트리가 무해**하다 — 이 행이 원래 적어 둔 문장 그대로다.
**E**(in-frame fatal 삭제)는 **elaborate 가 먼저 막는다**(태스크 바디 안 `fork` 는 E3009 · 양 백엔드
동일)라 소스에서 도달 불가. **F**(fall-through 가드 삭제)는 `continue` 가 바닥 증가를 건너뛰지만
**`wait fork` 를 담을 수 있는 모든 루프가 바닥에 닿는 `Branch` 블록을 갖는다**(그 fall-through 만으로
된 자기 루프는 구성 불가). **G**(게이트 스캔이 `wait fork` 뒤 resume 엣지를 안 따라간다)는 그 뒤에
놓을 수 있는 **거부 대상이 없다**(`Named` 는 미구성 · 사이드카 없는 `Call` 은 코퍼스 0).

전 스위트 **5465 green** · flip 발산 **0**.

#### 5.1-ap ✅✅ A4-d `disable fork` = **Phase A(V1) 완주** — 99.89% → **100.00%** (2026-08-16)

**+7 설계. 코퍼스 6,470 중 거부 0.** `simulate()` 호출 전부가 ③층 native 로 돈다.

⭐ **마지막 행도 위임이었다.** IEEE §9.6.3 의 kill 은 **넷 값을 하나도 안 읽는다** —
`Scheduler::k_disable_fork` 는 `activities`/`barriers` 를 전이적으로 걷고 §16.4 리포트를
`st.postponed` 에서 취소하는 것이 전부이고 둘 다 커널이 이미 빌리는 것이다 ⇒ tier-3 메서드는
**위임 한 줄**.

⭐⭐ **진짜로 없던 것은 이 함수가 아니라 dispatch choke 의 두 줄이었다** — `Scheduler::run_body` 가
하는 ⓐ **죽은 활동 드롭**(`disable fork` 는 아무것도 unschedule 하지 않는다 · 이미 큐·waiter·delay
wheel 에 들어간 재개 항목이 **도착해서 버려지는** 것이 설계다 ⇒ 큐 수술이 필요 없고 런타임 비용이
이 검사 하나다) ⓑ **`cur_aid`/`cur_gen` 세팅**(= kill set 의 **뿌리**). tier-3 은 둘 다 없었다.

⚠️⚠️ **그리고 ⓑ가 pre-existing silent-wrong 을 드러냈다 — 이 슬라이스와 무관한 것이다.**
§16.4 deferred 리포트는 `(marker_sid, cur_aid, cur_gen)` 로 키잉되는데 tier-3 이 그 쌍을 한 번도
세팅한 적이 없어 **모든 deferred 리포트가 활동 0 으로 파일**되고 있었다. 두 프로세스가 **같은
`assert #0` 문장**에 닿는 최소 설계에서 **`$error` 둘 중 하나가 사라진다**(실측 `errors=1` vs 2 ·
exit 0). A8-b 가 deferred assertion 을 배선한 이래 있던 결함이고, **A4-d 가 같은 두 줄을 자기
이유로 필요로 했기 때문에** 지금 드러났다. 앵커로 핀했다.

⚠️ **거부 핀 일곱이 걸렸고 전부 마지막 design 행을 이름으로 부르고 있었다.** 처리:

| 테스트 | 처리 |
|---|---|
| `native_gate::statement_level_families_reject` | 두 번째 절을 **positive 로 반전** |
| `native_gate::sidecar_families_reject_from_opts` | 통계 절을 `is_empty()` 로 |
| `native_gate::the_runtime_gate_is_exactly_design_and_storage` | **design-refused arm 이 소멸** → `(2,0,1)` 로 재핀 + 이유 기록 |
| `kernel_tests::s1d4a_refused_workers_are_loud_not_silent` | **자기 doc 이 예고한 대로 은퇴** → 이빨을 `gate_refused!` **소스 스캔**으로 이전 |
| `cli::obs::run_json_native_pins_the_reject_families` | 빈 map 으로 반전(설계는 유지 — 마지막 non-empty 를 만든 그 소스다) |
| `cli::obs::run_json_reports_native_fallback_on_a_refused_design` | **개명 + 반전** = `run_json_asks_for_native_and_gets_it`(A4-c 가 적어 둔 지시 그대로) |
| `cli::backend_flag::the_native_verdict_reports_scope_and_storage_separately` | design-refused arm 반전 + **scope/storage 분리 성질이 어디서 계속 시험되는지** 기록 |

⭐⭐⭐ **게이트의 세 층이 전부 비었다.** `gate_refused!` **매크로에 사이트 0**(17 → 0 · 매크로 삭제) ·
`systask_refusal` **집합 비었음**(6→4→2→0) · 실행기 거부 표 **비었음**(4→5→4→3→1→0) · design 행
**도달 가능한 것 0**. ⚠️ **"검사를 지웠다"가 아니다** — 세 함수와 소비자는 남아 있고 `_`-free match
가 새 종류를 강제하며, 오늘 핀하는 것은 **"지금 비어 있다"** 다.

**뮤테이션 4 · 사망 4.** ⚠️ **생존 하나가 있었고 그것도 눈먼 축이었다** — `set_cur_activity` 에서
**`cur_gen` 만** 떼면 앵커 셋을 통과한다. 판별에는 **활동 슬롯 재활용**이 필요하다: 끝난 fork 자식의
슬롯이 `free_activities` 로 돌아가 다음 fork 가 **같은 `aid` 를 gen+1 로** 받는데, deferred 리포트가
`(sid, aid, gen)` 로 키잉되는 것은 정확히 그 재활용 때문이다. 한 슬롯 안에서 `fork…join` 을 **두 번**
하고 두 arm 이 **같은 `assert #0`** 에 닿게 하자 즉시 사망(`errors=1` · `who=1` 소실). ⚠️⚠️ **이 축을
재는 테스트가 저장소에 하나도 없었다 — 엔진 쪽에도** (공유 스케줄러 코드라 **양 백엔드가 똑같이**
틀린다) ⇒ 앵커를 지어 넣었고, 그것이 `set_cur_activity` 를 한 철자로 유지하게 만드는 장치다.

전 스위트 **5469 green** · flip **6470/6470 = 100.00% · 발산 0**.

> **⇒ Phase A 종료. 다음은 Phase B(빌드 분리) — B1 전 스위트 native 초록 · B2 VM 삭제 · B3 `oracle`
> feature · **B4 제품 빌드에서 게이트 거부를 loud 로**(삭제만으론 안 생기는 사다리 상승) · B5 CI 축.**
> 해설·용어·전체 서사 = **[study/02](study/02-v1-native-coverage.md)**.

#### 5.1-e ⚠️⚠️ 오라클 부식 — **V1 이 자기 오라클을 무디게 한다**(실측)

> **✅ 2026-08-16 갱신 — 이 절의 처방이 Phase B 에서 구조가 됐다.** 아래 결론(*"interp 의 값은
> 오라클이 아니라 ⓐ 이분 도구 ⓑ 읽을 수 있는 정본"*)이 이제 **빌드 경계**로 존재한다:
> `oracle` feature(기본 ON)가 `interp`·`vm` 을 들고, **제품 형태에는 없다**. 즉 *"영구 오라클"* 이라는
> 문장은 이미 사실이 아니고 — 제품에는 오라클이 없다 — **절대 앵커가 유일한 방어선**이다.
> ⚠️ 그리고 flip 런의 방향이 뒤집혔다(§5.1-aq): 기본이 native 이므로 **`native → vm`** 으로 물어야
> 오라클 축이 계속 시험된다. **Phase C 는 이 절을 코드가 아니라 정책으로 확정하는 일이다.**


§5.1 의 원래 근거(*"인터프리터를 영구 오라클로 남긴다"*)는 **V1 자신이 반증하는 중**이다. V1 의
방법이 **위임**이기 때문 — 최근 5 슬라이스 중 4개가 커널 코드 0줄이고 전부 native 를 공유
코드(`SimState`·`builtins::dispatch`·`eval::*`)로 보냈다.

| 실측 | 결과 |
|---|---|
| §4.5.330/331 | 공유 함수 뮤테이션 넷이 **차분을 전부 통과** · `arith_bits` 오라클 핀에서만 사망 |
| §4.5.337 ⓑ | 공유 dispatch 뮤테이션 둘이 **엔진 게이트 전부 통과** · CLI 절대 앵커에서만 사망 |
| 슬라이스 3b | string 반쪽에서 **두 백엔드가 똑같이 `[ ][\u{1}][ ] len=0`** — 차분 초록, 설계 틀림 |

⭐⭐ **native 가 interp 에 위임할수록 차분은 자기 자신을 비교한다. 커버리지 100% = 오라클 이빨 0%.**

⇒ **대체재는 "세 번째 실행기" 가 아니다**: **절대 앵커**(hand-IEEE 기대출력 · §4.5.337 이 바로 이
이유로 발명)와 **외부 오라클**(iverilog/verilator, subset 한정)이다. interp 의 잔여 가치는
오라클이 아니라 **ⓐ 버그 이분 도구 ⓑ 의미의 읽을 수 있는 정본** — 2,131줄이면 그 값어치는 하지만
**"영구 오라클" 이라 부르는 것을 그만둬야** 그 문장에 기대어 절대 앵커를 안 짓는 일이 없어진다.

⚠️ **의미 분리는 금지** — `Cargo.toml` 이 이미 그 값을 적어 두고 있다(*"식 의미의 세 번째 구현 …
두 번째 것이 첫 번째에서 독립적으로, 조용히, 네 갈래로 이탈했다"*, §4.5.279). byte-identity 가
**구조적**인 이유는 `compute_effect`/`apply_effect` 가 `Kernel` 제네릭이고 구현자가 **정확히
둘**(`Scheduler`·`NativeKernel`)이기 때문이다. 허용되는 분리는 **역할**(C1)과 **빌드**(B3)뿐이다.

---

| 단계 | 무엇 | 게이트 / 중단 판정 |
|---|---|---|
| **V0** ✅ | **커버리지 격차 측정 (2026-08-10 · §4.5.336).** 기본 백엔드를 `Native` 로 flip 하고 전 스위트를 돌렸다. **결과 = 아래 두 표.** 스캐폴드는 되돌렸다(트리 변경 0) | — (계기). **이 숫자가 V1 의 슬라이스 목록이다** |
| **V1** ✅ **완료 (2026-08-16)** | **커버리지 확장 = Phase A.** §5.1-b 의 측정된 순서대로 30여 슬라이스 — **54.7% → 100.00%**(코퍼스 6,470 중 거부 0 · flip 발산 0). 슬라이스별 상세 = §5.1-g~-ap · 해설 = [study/02](study/02-v1-native-coverage.md) | 슬라이스마다 **VM 과 바이트 동일 + 절대 앵커**(§5.1-e) |
| **V2** ✅ **완료 (2026-08-16)** | **= Phase B(빌드 분리).** 기본값을 native 로(§5.1-aq) · 교체를 경고로(§5.1-ar) · **`oracle` feature**(기본 ON)로 제품 표면을 native 하나로(§5.1-as) · 제품 빌드에서 거부를 치명으로(§5.1-at). ⚠️ **삭제한 줄은 0** — 착수 전 측정이 옛 표적(5,430줄 삭제·`exec/` 감싸기)을 뒤집었다(§5.1-b2) | 기본 5,470 green · **제품 형태 lib 147 green** · clippy 양쪽 0 |
| **V3** ✅ **완료 (2026-08-17)** | **= Phase C.** `--backend interp` 를 **테스트 도구**로 명시(제품 빌드엔 변형 자체가 없다) · **성능 최적화 영구 제외**를 `Backend::Interpreter` doc 에 규칙으로 · §5.1-e 를 빌드의 성질로 확정. ⚠️ 그 과정에서 **`--help` 한 문단이 통째로 거짓**임을 발견해 다시 썼다(§5.1-au) | 5,470 green · 제품 형태 147 green · 도움말 핀 **강화** |

#### 5.1-a V0 측정 결과 (2026-08-10)

**전 스위트를 `Backend::Native` 기본으로 돌린 결과 — 5377 중 5374 통과, 실패 3건은 전부
"기본 백엔드가 vm 이다" 를 단언하는 테스트**(`the_default_backend_is_the_vm`,
`run_json_codegen_*`)다. **출력 발산 0.** 즉 오늘의 ③층은 자기가 받아들이는 것에 대해 이미
바이트 정확하고, 남은 것은 **전부 커버리지**다.

| | 값 |
|---|---|
| `simulate()` 호출 | **6,251** |
| 그중 **네이티브 실행** | **3,417 (54.7%)** |
| fallback | 2,834 (45.3%) — design 2,230 · storage 496 · executor 108 |
| 발산 | **0** (실패 3건은 백엔드 이름 핀) |

⚠️ **테스트 단위 귀속은 못 쓴다** — fallback 2,834 중 **2,656(93.7%)이 CLI 서브프로세스**라
`cli::*` 통합 테스트가 전부 바이너리 경로 하나로 뭉친다. 단위는 **`simulate()` 호출**이다.

#### 5.1-b V1 슬라이스 순서 — **측정이 정했다**

⭐⭐ **행이 아니라 짝을 닫아야 한다.** design 게이트와 storage 게이트가 **같은 기능을 두 번
이름 부른다** — `string` 넷은 `D:string`(설계 행)이면서 동시에 `S:heap-slot`(저장 거부)이라
한쪽만 닫으면 **이득이 정확히 0**이다(`D:string` 369회 발화, 단독 원인 **0회** · `D:real` 74회,
단독 **0회**). 그래서 슬라이스의 단위는 행이 아니라 **한 기능이 걸린 모든 게이트의 집합**이다.

| # | 슬라이스 | +호출 | 누적 커버리지 |
|---|---|---|---|
| — | (현재) | — | 3,417 / 54.7% |
| 1 | ✅ **SVA** — `sva` 행 완료(§4.5.337, +760 → **66.8%**) · `deferred_assert`(+14)는 별개 | +774 | **67.0%** |
| 2 | **heap 저장 + 네 종류** — **넷으로 갈린다(§5.1-c)** · ✅ **2a(`dyn_array`, +187)·2b(`string`, +164) 완료** · 2c/2d 남음 | +560 | **76.0%** |
| 3 | **서브루틴 프레임** (`task-frame`·`call-stmt`·`call-in-systask-arg`·`frame-reads-module-net`·`frame-stmt` …) | +712 | **87.4%** |
| 4 | `stmt_effect` | +241 | 91.2% |
| 5 | **class / OOP / CRV** | +163 | 93.9% |
| 6 | fork (`fork`+`disable_fork`) | +104 | 95.5% |
| 7 | 거부 시스템태스크(VCD 추가·`$monitor`/`$strobe`·파일) | +83 | 96.8% |
| 8 | `real` (`real`+`real-slot`) | +72 | 98.0% |
| 9 | functional coverage | +64 | 99.0% |
| 10 | `force`/`release` | +17 | 99.3% |
| 11 | clocking 블록 | +16 | 99.6% |
| 12 | G2 probe/stage rail | +12 | 99.7% |
| 13 | `final` 블록 | +8 | 99.9% |
| 14 | file-directed 태스크 | +8 | **100%** |

읽는 법: **"+호출" 은 그 위 슬라이스가 전부 닫혔다는 전제의 한계이득**이다(greedy). 단독으로
닫았을 때의 이득은 다르다 — 예: 3번을 **혼자** 닫으면 +545, 위 둘 뒤에 닫으면 +712(heap·sva 와
겹치던 설계가 함께 풀린다). `fork`·`file_directed` 는 **단독 이득이 0**이다(항상 다른 것과 함께
발화).

⭐ **상위 셋이 87.4% 를 산다.** 그리고 그 셋은 vita 의 Phase-2/3+ 자산 그 자체다 —
SVA·힙 자료구조·서브루틴 프레임.

#### 5.1-c 슬라이스 2(heap) 그라운딩 — **넷으로 갈리고, 슬라이스 1과 달리 진짜 작업이다** (2026-08-11)

**⭐ 값은 넷 슬롯이 아니라 `dyn_heap[net]` 에 있다** — 핸들이 아니라 **넷 id 로 키잉된 별도 힙**
이고, `NativeKernel` 은 이미 그 `SimState` 를 빌린다. 그래서 이것은 "새 저장소를 짓는 일" 이
아니라 **라우팅**이다. 엔진 `read_net` 이 이미 그 모양이다 — 넷마다 비트맵으로
`frame_local` → 프레임 · `dyn_is_handle` → `dyn_read`(힙) · 그 외 → 평면.

**측정된 sub-slice (각각 독립 · 합 560):**

| sub | 닫을 것 | +호출 |
|---|---|---|
| **2a** | `dyn_array` + `heap-slot` | **+187** ✅ |
| **2b** | `string` + `heap-slot` | **+164** ✅ |
| **2c** | `queue` + `heap-slot` | **+96** ✅ (`queue_ops` 동반) |
| **2d** | `assoc` + `heap-slot` | **+32** ✅ |
| — | `queue_ops` · `handle_copy` | **단독 +0** — 항상 자기 종류와 함께 발화하므로 딸려 온다 |

**⭐⭐ 그리고 그라운딩이 비대칭 하나를 찾았다 — 이것이 슬라이스의 크기를 정한다.**
`is_frame_local` 라우터는 **`kernel.rs` 에만 있고 `write.rs` 에는 없다**: S3a 가 프레임 호출을
**통째로 엔진 실행기에 위임**했으므로 프레임 쓰기가 tier-3 의 쓰기 퍼널에 **도달하지 않기**
때문이다. 그런데 heap 쓰기(`q.push_back(x)` · `s = "hi"` · `arr[i] = v`)는 **도달한다** — census
가 그것을 확인한다(heap-only 설계 560건의 blocker 집합에 **`stmt_effect` 가 0건**이므로 그 쓰기는
특수 효과가 아니라 평범한 퍼널을 탄다).

**⇒ 슬라이스 2 는 슬라이스 1처럼 "리더에 arm 하나" 가 아니다.** 필요한 것은 ⓐ
`NetArena::buildable` 이 그 종류를 받고 평면 슬롯을 **안 만드는 것**, ⓑ 복합 리더의 **세 번째
arm**, ⓒ **쓰기 쪽 라우터**(새 기계장치 — 오늘 대응물이 없다), ⓓ VCD·dirty·엣지 채널이 그
넷들을 어떻게 다루는지(엔진은 힙 넷을 파형에 어떻게 내는가). ⓒ 가 이 슬라이스의 본체다.

⚠️ **2a 부터.** 가장 크고(+187) 자기완결적이며, 나머지 셋이 재사용할 라우팅 패턴을 세운다.

##### 2a 착수 기록 (2026-08-11) — **쓰기 퍼널은 섰고, 게이트는 아직 닫혀 있다**

**끝난 것**: `NativeKernel::write_routed` — tier-3 의 **단일 쓰기 퍼널**(7 사이트 → 1, 커밋
`38cf264`, 소스 스캔 테스트가 퍼널로 유지). 이것이 2a~2d 전부의 선행조건이다.

**착수해서 되돌린 것**: 리더/라이터 dyn arm + `NetArena::buildable` 의 `DynArray` admission +
`design_eligibility` 의 `dyn_array` 행 개방. **차분이 즉시 발산을 잡았고, 되돌렸다.**

⭐⭐ **그 발산이 2a 의 진짜 크기를 알려 줬다 — 힙 접근은 `read_net` 만이 아니다.**
`q = new[4];` **직후에 `q.size()` 가 이미 `x`** 였다(값 셋도 x, `q[0]` 만 우연히 0). 즉 배열
메서드(`size`/`delete`/`push_back`…)와 `new[]` 는 `EvalCtx`/sysfunc 경로로 가는데 **`NetReader`
트레이트에 힙 표면이 아예 없다** — `dispatch_with` 의 `DynNew` arm 도 `sched.eval(a)` 로
**엔진 평가기**를 쓴다(§4.5.293 이 포맷터에 대해 고친 것과 같은 클래스: 태스크 자기 인자는
스레드된 리더를 거치지 않는다).

⇒ **2a 는 arm 둘이 아니라 세 번째 축이 더 있다**: 힙을 만지는 **평가기 표면**(배열 메서드 ·
`new[]` 인자 · `.size()`)이 tier-3 의 리더를 통과하도록 하는 것. 그것 없이 게이트를 열면
**값이 틀린 채 네이티브로 도는 silent-wrong** 이 된다(실측했고, 그래서 닫았다).

##### 2a 진행 2 (2026-08-11) — census 가 나왔고, **내 앞 진단 하나가 틀렸다**

⭐⭐ **`NetReader` 에는 힙 표면이 있었다** — 앞 항목의 *"표면이 아예 없다"* 는 **틀렸다**.
`dyn_size`/`dyn_values`/`str_bytes`/assoc 6종 … 이 전부 트레이트에 있고, **기본 구현이 있다.**
tier-3 은 그것을 **말없이 상속**하고 있었다: 21 메서드 중 오버라이드가 **7개**뿐이었고, 나머지
14개의 기본값은 전부 **그럴듯한 값**을 낸다(`None`→호출자가 X-poison · `false`="assoc 아님" ·
`xs`). ⇒ **게이트가 닫혀 있는 동안만 무해하고, 행을 여는 순간 조용한 오답이 된다.**

**고친 것**: 복합 리더를 **전면적(total)** 으로 만들었다(14 위임 — 전부 `SimState` 에만 있는
상태라 게이트와 무관하게 옳다) + **구조 핀** `the_composite_reader_overrides_every_netreader_method`
가 그것을 유지한다(빠진 메서드는 **호출 지점이 없어** 런타임 단언이 불가능하다).

⚠️ **그런데 게이트를 다시 열자 여전히 발산했다.** `q0=7` 은 맞고 `q1`·`size` 만 x, 그리고
**native 만 exit 1** — E4002 범위 진단이고 그것을 세는 것은 **아레나뿐**이다. ⇒ 그 읽기는 복합
리더를 **거치지 않고** 아레나로 갔다.

⚠️⚠️ **그리고 내가 지목한 범인은 틀렸다.** `wprog`의 `LoadIdx` 를 *"kind 가드가 없다"* 고 적었으나
(커밋 `0628111` 메시지 포함), **`WOp::LoadIdx` 는 `Expr::Signal` arm 안에서 방출되고 그 arm 은
`Wire|Reg|Logic|Integer` 화이트리스트를 arm 진입부에 갖고 있다** — 상수 인덱스와 런타임 인덱스가
같은 가드 아래 있다. 그러므로 `wprog` 는 우회 경로가 **아니다**. 우회 지점은 아직 **미확정**이다.

**해결(2026-08-11) — 후보 ⓑ**: 라우팅을 **`SimState::eval_expr_with`** 에 뒀다. 그 함수가
`&self`(= 힙 소유자)와 `nets`(= 호출자의 store)를 **이미 함께 들고 있는 유일한 프레임**이라
대여 충돌이 아예 없다(ⓒ 는 `&sched.st` 와 `&mut sched` 가 겹쳐 **컴파일되지 않고**, ⓐ 는
아레나에 수명을 오염시킨다). 리더가 `routes_heap_to_state()` 로 **요청할 때만** 켜지므로 엔진
경로는 `if false` = 기계적 불변(§4.5.314 의 opt-in 규칙). ⚠️ 재귀로 쓰면 monomorphization 이
`HeapRouted<HeapRouted<…>>` 를 무한 전개하므로 본문을 `eval_expr_inner` 로 분리했다.
⭐ 그 함수의 호출자가 **넷뿐이고 전부 builtins 의 인자 경로**라, §4.5.294 가 따로 빼야 했던
`eval_task_arg` 까지 **한 자리로 덮인다**.

**⇒ 슬라이스 2a 완료.** `dyn_array` 는 CORE, 원소 정제 둘(`dyn_elem_real`/`dyn_elem_string`)은
자기 이름으로 거부. 전 스위트에서 **`assert_owns` 패닉 0건** = 남은 우회 경로 없음.

##### 2b (`string`, +164) 완료 (2026-08-11)

2a 가 지은 기계장치를 **그대로** 썼다 — `buildable` 에서 `String` 을 열었을 뿐이고 새 라우팅은
없다. ⭐ **계기가 이번엔 빌드 시점에 잡았다**: 아레나의 t0 init 루프가 *"선언 init 이 폭 위의
비트를 나른다"* 로 터졌다 — `string` 의 init 은 **패킹된 리터럴**이고 슬롯은 원소 폭으로
잡혀 있다. 죽은 슬롯을 그것으로 채울 이유가 없으므로 **heap 넷은 t0 슬롯 init 을 건너뛴다**
(진짜 t0 값은 힙의 것 = IEEE §7.5.2 의 `""`).

⚠️ 그 assert 는 **이 슬라이스가 지은 것이 아니라 원래 있던 불변식**이다 — 2a 의 `assert_owns`
와 함께, 아레나가 자기 계약을 말하게 해 둔 것이 두 번 연속으로 표적을 지목했다.

##### 2c (`queue` + `queue_ops`, +96) 완료 (2026-08-11) — **그리고 2a·2b 가 틀린 채 배송돼 있었다**

**⭐⭐ 이 슬라이스의 산출은 queue 가 아니라 발견이다.** 착수 전 차분이 **2a·2b 가 만든 라이브
silent-wrong** 을 잡았다 — `builtins::dispatch` 는 대체 store 를 **파라미터로 받지만 포맷터를
거치는 arm 만** 그것을 쓰고, 나머지는 `Scheduler::eval` / `eval_ctx_top` / `assoc_key_of` 로
**`SimState` 자기 넷** 위에 `EvalCtx` 를 짓는다. 그것이 네이티브 런이 **한 번도 안 쓰는** store 다.
힙 종류가 전부 거부되던 동안엔 그 arm 들이 도달 불가였고, 2a/2b/2c 가 넷을 도달 가능하게 만들었다:

| 철자 | native | VM |
|---|---|---|
| `d = new[n]` (n = 넷) | `size=0` | `size=3` |
| `s.itoa(v)` (v = 넷) | `s=0` | `s=200` |
| `q.push_back(a)` (a = 넷) | `q[0]=x` | `q[0]=42` |
| `r = q[a:b]` (a·b = 넷) | 빈 큐 | `size=2` |

⭐ **판별자는 인자가 넷이라는 것 하나뿐이다** — 바로 옆의 `q.insert(i, 32'd99)` 는 리터럴이라
맞았고, 2a·2b 의 차분 행이 **전부 리터럴 인자**라 두 스위트가 초록이었다. **슬라이스가 태스크를
admit 하면 그 태스크의 인자는 두 번째 store 읽기이고 자기 행이 필요하다.**

**고친 것**: `eval_task_arg`(§4.5.294 가 이미 지어 둔 것)로 넷을 라우팅하고, 원소 폭으로
context-size 하는 두 mutator 를 위해 **폭 쌍둥이** `eval_task_arg_ctx` +
`SimState::eval_ctx_with_reader` 를 지었다. 둘 다 `None` 팔이 **예전 그 호출 자체**라 엔진 경로는
기계적 불변(§4.5.314 opt-in 규칙). `run_queue_slice` 는 리더를 파라미터로 받는다.

**구조 핀** `every_untreaded_store_read_in_builtins_sits_behind_a_reject_row` — 남은 raw 읽기
13개를 **파일별 개수 + 각각을 막는 행 이름**으로 고정한다. 행을 여는 슬라이스는 **여기서 먼저**
깨진다. (남은 것: `dispatch.rs` 4 = file_directed·**assoc key(2d 소관)**·`$dumplimit`·`$fclose` ·
`crv_draw.rs` 6 = class 행 · `render.rs` 2 = stage 행 · `queues_io.rs` 1 = seam 자신.)

**queue 자체는 행 둘**: `buildable` 의 `NetKind::Queue` 와 `queue_ops`(= `queue_slice_stmts` +
`queue_bounds`). 후자가 열린 이유는 **두 테이블 다 `SimState` 에 살고 tier-3 이 이미 공유하는
코드가 읽기 때문**이다(`enforce_queue_bound` 는 넷 store 를 아예 안 만지는 `&self` 힙 메서드).

⚠️ **`k_queue_pop` 을 막는 행이 바뀌었다** — 예전 이유(*"NetKind 스캔이 queue 저장을 거부"*)는
이제 거짓이고, 실제로 막는 것은 `stmt_effect` 다(`x = q.pop_front()` 는 `rhs_is_stmt_effect` 가
세는 `BlockingAssign`). 측정으로 확인: pop 없는 queue 설계는 오늘 네이티브로 돈다.

⚠️ **거부 핀 셋이 공허해질 뻔했다** — `native_gate.rs`·`cli/obs.rs`·`cli/backend_flag.rs` 가
전부 `string s; int q[$]` 를 "거부되는 설계" 로 쓰고 있었다. 전부 `real` 로 옮겼다(§5.1-b 슬라이스
10 이라 한동안 안전하고, **양쪽 게이트 절반이 자기 이름으로 거부**하는 유일한 남은 종류다).

**뮤테이션 11/11 사망** — 그런데 ⭐⭐ **배터리가 처음엔 셋을 "핀이 잡았다" 로 보고했고 그것이
발견이었다.** 두 원인이 겹쳐 있었다:

1. ⚠️ **`cargo nextest` 는 기본이 fail-fast** 라 첫 실패에서 나머지를 취소한다 — 기록된 killer 는
   **가장 먼저 도는 테스트 하나**였다. `--no-fail-fast` 로 다시 물으니 셋 중 둘은 **코퍼스가 이미
   잡고 있었다**.
2. ⭐⭐ 남은 하나(`run_queue_slice` 의 리더 되돌리기)는 **진짜로 코퍼스가 눈멀어 있었다** —
   `build_with_opts` 가 `queue_slice_stmts`/`queue_bounds` 를 **설치하지 않아서** `r = q[a:b]` 는
   슬라이스가 아니었고 `int bq[$:2]` 는 bound 가 아니었다. **§4.5.337 이 SVA 로 겪은 함정의 재발**
   (그때는 `assert_ctl`). 설치하자 코퍼스가 죽인다.

⇒ **소스 스캔 핀은 변경 탐지기이지 동작 테스트가 아니다.** 핀만 잡는 뮤테이션이 남으면 그것은
게이트가 강하다는 신호가 아니라 **그 행이 공허하다는 신호**다.

⭐ **파형 축도 이번에 닫았다** — 그라운딩이 열어 둔 질문 ⓓ(*"VCD·dirty·엣지 채널이 힙 넷을 어떻게
다루는가"*)의 답은 **양쪽 백엔드에서 셋 다 밖**이고(넷 dirty 채널 없음 = dyn 선례), 옆의 평면
넷은 그대로 변화를 낸다. `$dumpvars` 를 든 queue+string 행으로 **VCD 바이트 비교**까지 영구화.

<!-- 해결됨: 아래는 당시의 후보 목록 -->

**당시의 세 후보**(구현이 아니라 설계 선택):
ⓐ 아레나가 힙을 빌려 스스로 라우팅하게 한다(수명 문제) · ⓑ `dispatch_with` 가 힙 넷을 `nets`
보다 먼저 `sched.st` 로 보낸다(리더 호출이 `eval` 깊숙이 있어 진입점이 애매) · ⓒ 포맷 경로 전용
복합 리더를 `&NetArena + &SimState` 로 만든다(`sched` 의 가변 대여와 겹치지 않게).
**게이트는 그 결정 전까지 닫아 둔다.**


##### 2d (`assoc` + `AssocStr`) 완료 — **슬라이스 2 닫힘 · 커버리지 재측정 54.66% → 72.75%** (2026-08-11)

**⭐ 유일하게 새 arm 이 필요했던 종류.** dyn/string/queue 는 전부 공유 `dyn_write` 로 갔지만
**assoc 키는 i64(또는 바이트열)라 `(offset, word)` u32 쌍을 못 탄다** — `resolve_offsets` 가
키를 `Offsets::AssocKey`/`AssocStrKey` 로 **대역 밖**에 실어 보내고 `as_slice()` 는 `&[]` 를 낸다.
그래서 내 `unwrap_or((0,0))` 이 **모든 키를 조용히 0 으로** 만든 뒤 `dyn_write` 의 loud-ignore arm
에 넘기고 있었다: `aa[3]=7; aa[9]=11` 이 아무것도 저장 못 하고 `x x 0 0`(VM `7 11 2 1`).
수정 = `write_routed` 가 `SimState::write_lvalue` 와 **같은 지점에서 같은 두 메서드로** 분기.

**키 읽기 둘도 배선했다**(`assoc_key_arg`/`assoc_str_key_arg`). ⭐ 재진술을 피하려고 키 규칙을
**추출**했다 — `assoc_key_eval_ctx`(≥64비트 평가 문맥)·`assoc_key_of_value`·`assoc_str_key_of_value`
가 정본이고 `EvalCtx::assoc_key` 는 이제 그 위의 한 줄이다. 그리고 옛 진입점
`Scheduler::assoc_key_of`/`assoc_str_key_of` 는 **남기지 않고 지웠다**(두 번째 철자가 하나의 편집
만큼 떨어져 있으면 언젠가 **쓰기 lane 과 다른 엔트리를 가리킨다**).

⚠️ **string 키는 배선 전에도 VM 과 일치했는데 그건 운이었다** — `string` 키 자체가 힙 넷이라
엔진 store 가 마침 그것을 들고 있었다. **packed 키**(`reg [15:0] k = "hi"`)는 평면 store 를 읽어
틀린다. 그래서 차분 행을 packed 키로 지었다.

⚠️ **내 구조 핀에 패턴 구멍이 있었다** — `sched.assoc_key_of(` 가 스무 줄 위의 `assoc_str_key_of`
를 안 셌다. **한 식구의 이름 하나를 적은 패턴은 스캔이 아니라 화이트리스트다.**

##### ⭐⭐ 그리고 flip 런이 **2a 이래 계속 틀려 있던 것**을 찾았다 — concat lvalue

`write_routed` 는 `if let [c] = lhs.chunks.as_slice()` 로 **lvalue 전체가 한 청크일 때만** 라우팅
하는데, 엔진은 **청크마다**(`write_chunk` 의 첫 질문이 `dyn_is_handle[net]`) 라우팅한다. 그래서
`{d[0], x} = 8'hAB` 이 두 청크를 다 아레나로 보냈고 힙 청크가 `assert_owns` 에 닿았다.

⚠️ **코퍼스는 이것을 원리적으로 못 본다** — 그 두 테스트는 기본 백엔드(vm)로 돌기 때문에
아레나에 아예 도달하지 않는다. **전 스위트 백엔드 flip 만이 신호였다.**

**라우팅하지 않고 거부했다**(storage 게이트, 자기 이름으로). 라우팅하려면 소스를 청크별로 쪼개
서로 다른 store 로 보내야 하는데 **그 분할 규칙은 이미 `NetArena::write_lvalue` 안에 있고**,
라우터에 두 번째 철자를 두는 것이 §4.5.279 클래스다. **correct-support 는 퍼널에 청크별 탈출구를
주는 별도 슬라이스** (아래 §5.1-d). 비용은 고르기 전에 쟀다 — 전 스위트에서 **설계 3건**.

⚠️ `string` 은 여기 안 넣었다: `{s, x} = …` 는 **어느 게이트에도 도달 안 한다**(elaborate 가 이미
거부). 넣었으면 위 단계가 한 일을 아래 단계가 한다고 주장하는 **공허한 행**이었다.

##### ⭐⭐ 슬라이스 2 종료 — 재측정 (2026-08-11)

**투영이 아니라 실측이다.** V0 의 계기를 다시 세워(`simulate()` 마다 세 층 판정을 한 줄 `write_all`
로 기록) 전 스위트를 돌렸다:

| | V0 (2026-08-10) | 슬라이스 1+2 후 |
|---|---|---|
| `simulate()` 호출 | 6,251 | **6,290** |
| 네이티브 | 3,417 (**54.66%**) | **4,576 (72.75%)** |
| 발산 | 0 | **0** (기본 백엔드 flip: 5385 중 5382 통과, 실패 3건은 **전부** *"기본 백엔드가 vm"* 핀) |

⚠️ **투영은 ≈74.0% 였고 실측은 72.75% 다** — census 는 실측이지만 그 위의 greedy 누적은 산술이고,
슬라이스가 자기 거부 행을 새로 만들면(concat lvalue −3) 어긋난다. **슬라이스마다 투영을 적되
슬라이스 묶음이 닫힐 때 재측정한다.**

**다음 표적은 census 가 정한다**(첫 blocker 기준이라 과소평가임에 주의):

| 잔여 blocker | 건수 |
|---|---|
| `task frames (Terminator::Call)`: S3b | **391** |
| ~~`dyn_elem_string`~~ **✅ 슬라이스 3b 에서 해소(+179, `dyn_elem_real` 동반)** | ~~206~~ |
| ~~`a call in a system-task argument`: S3b~~ **✅ 슬라이스 3a 에서 해소(+205)** | ~~203~~ |
| `stmt_effect` | **200** |
| `class` | 164 · `fork` 94 · `handle_copy` 87 · `real` 69 · `coverage` 64 |

⭐ **상위 셋 중 둘이 같은 것**(서브루틴 프레임 = V1 슬라이스 3) 이고, **`dyn_elem_string` 206 은
슬라이스 2 가 일부러 남긴 원소 정제**라 heap 가족의 자연스러운 다음 조각이다.


#### 5.1-d 슬라이스 3a — **시스템태스크 인자 속의 호출**: 슬라이스 2 가 이미 지어 둔 seam (2026-08-12)

**커버리지 72.75% → 75.99%(+205 호출) · 커널 코드 0줄 · 지운 것은 `native::frames` 의 행 하나.**

⭐⭐ **그 행의 거부 이유는 자기 주석에 정확히 적혀 있었고, 그 이유가 이미 거짓이었다.**
*"`k_dispatch_systask` 는 `&mut Scheduler` 를 들고 있어서 `dispatch` 에 아레나를 **혼자** 넘기고
`&SimState` 를 composite 로 함께 빌려줄 수 없다"* — 그래서 `$display("%0d", f(x))` 의 호출이
`NetArena::eval_call`(**loud panic**)에 닿았다. 그런데 **V1 슬라이스 2 가 한 층 아래에 composite 를
지어 뒀다**: `SimState::eval_expr_with` 가 힙 넷을 라우팅하려고 리더를 `HeapRouted` 로 감싸는데,
**그 래퍼가 두 store 를 다 든다.** 그 `st` 쪽에서 호출 가족을 답하면 끝이다 —
`NativeKernel` 이 한 층 위에서 주는 답과 **같은 답**(`SimState::run_frame_call`).

⇒ **다른 목적으로 지은 seam 이 이 행을 무효화했고, 행은 그것을 몰랐다.**

**호출 가족은 넷이다** — `eval_call` 하나가 아니라 `resolve_virtual_call`·`formal_width`·
`formal_is_string` 도 같이 간다. 전부 `st` 로 보냈다.

⚠️⚠️ **그리고 내가 그 셋에 대해 쓴 주장을 뮤테이션이 반증했다.** 나는 *"아레나의 답이 패닉이
아니라 트레이트 기본값이라 조용히 틀린다 — `narrow(8'hFE)` 가 15 인 것은 formal 의 선언 4비트가
적용될 때뿐"* 이라 적고 전용 차분 행까지 지었는데, **뮤테이션 셋이 전부 생존했다.** 실측:
셋을 아레나로 되돌려도 narrow·widening-signed·string formal 이 **바이트 동일**하고, 결정적으로
**적대적인 `formal_width`(모든 formal 에 `Some((1,false))`)를 줘도 출력이 안 변한다.**
`eval_core` 의 강제는 **pre-sizing** 이고 `run_frame_call` 이 자기 메타데이터로 다시 바인딩한다 —
**리더의 답은 덮어써진다.**

⇒ **셋은 오늘 동치 뮤테이션이다.** 그래도 `st` 로 보낸다 — 커널이 한 층 위에서 주는 것과 **같은
답**이고, `NativeKernel::resolve_virtual_call` 이 이미 적어 둔 이유(*"그 행이 움직이는 날 한 줄짜리
정답이 조용한 오답보다 낫다"*)가 그대로 적용된다. ⚠️ **뮤테이션 둘을 동시에 걸면 서로를 가린다** —
B·C 를 함께 적용했을 때 `formal_is_string=false` 가 `formal_width` 의 1비트 답을 우회시켰다.

**게이트 쪽 정리**: 그 행을 증명하던 refusal 설계 3개를 지우는 대신 **positive 테스트로 뒤집었다**
(`a_call_in_a_system_task_argument_is_admitted`) — 줄어드는 벡터는 *"키가 사라졌다"* 를 보일 뿐
*"그 형태가 돈다"* 를 안 보인다. 그리고 `frames.rs` 모듈 doc 의 *"둘 다 S3b"* 문장을 실제 상태로
정정했다(**남은 것은 `schedule_delayed_cas` 하나** — 그 경로는 `eval_expr_with` 를 안 지난다).

**발산 0** — 기본 백엔드 flip 에서 5387 중 5384 통과, 실패 3건은 전부 *"기본 백엔드가 vm"* 핀.


##### 슬라이스 3b — **heap 원소 정제 둘**: 슬라이스 2a 가 남긴 보수성 (2026-08-12)

**커버리지 75.99% → 78.73%(+179 호출) · 커널 코드 0줄** · 지운 것은 `design_eligibility` 의 행 둘
(`dyn_elem_string`·`dyn_elem_real`).

슬라이스 2a 는 **컨테이너**만 열고 원소 정제 둘을 일부러 남겼다 — *"`string s[]` 원소는 바이트열,
`real r[]` 원소는 f64 이고 둘 다 컨테이너 행이 측정된 비트벡터 원소가 아니다"*. 재보니 **두 lane 이
전부 `SimState` 자기 힙 메서드 안에 산다**(`coerce_dyn_elem`·`alloc_dyn_array`·`dyn_read`/`dyn_write`)
— 슬라이스 2 가 이미 모든 힙 접근을 그리로 라우팅하므로, **정제는 컨테이너만큼이나 보수적이었다.**

##### ⚠️⚠️ 그리고 하네스가 **세 번째로** 같은 함정을 밟았다 — 이번엔 두 실패 모드가 동시에

`build_with_opts` 가 `string_elem_dyn_nets`/`real_elem_dyn_nets` 를 설치하지 않았다:

* **string 반쪽은 조용히 공허** — 두 백엔드가 **똑같이** `[ ][\u{1}][ ] len=0` 을 찍었다.
  설계가 말하는 것과 다른 것을 재면서 **완벽히 일치**한다.
* **real 반쪽은 실제로 발산** — VM `2.000000` / native `1.500000`. 핸들이 `is_real` 이 아니라
  원소 강제가 비트 resize 로 떨어지고 두 경로가 그 지점에 다르게 도달한다.

⇒ **string 만 있는 슬라이스였다면 초록으로 배송됐다.** 그래서 사이드카를 설치하고, 값 자체를
고정하는 **절대 앵커**(`heap_element_refinements_have_their_ieee_defaults_and_values`)를 지었다 —
`new[]` 의 IEEE §7.5.2 원소 기본값(`""` / `0.0`)까지 포함해서. **차분은 이 선을 원리적으로 못 지킨다.**

⭐ **일반 규칙**: 사이드카는 선택적 문맥이 아니라 **소스의 의미의 일부**다. 코퍼스가 철자할 수 있는
것은 전부 자기 테이블을 하네스에 가져야 한다.

##### 그리고 task frames 는 **재보고 미뤘다** (391 중 143 만 subset)

`Terminator::Call` 은 tier-3 워크에 arm 이 아예 없다. 엔진의 그 팔은 **두 갈래** —
suspendable 태스크(콜스택 push·park/resume·dyn stash·재귀 깊이)와 **subset 태스크의 동기 실행**
(입력 평가 → `run_task_call` → **호출자 lvalue 로 copy-out**). 후자만이면 슬라이스 3a 와 같은
라우팅 문제(copy-out 이 `sched.st.write_lvalue` = 엔진 store → `write_routed` 필요)다.

**착수 전에 쟀다**: 이 행에 걸린 **391 중 143(36.6%) 만 suspendable 태스크가 0개**다
(`tasks=1 susp=1` 이 196 · `tasks=1 susp=0` 이 133). ⇒ **subset 만 지어도 상한이 +143** 이고,
그것도 프로세스 바디에 `Terminator::Call` arm 과 `Kernel` seam 을 새로 지어야 한다.
**`stmt_effect`(205)·`class`(164) 보다 크지 않으므로 순서를 뒤로 미룬다.**

⚠️ **V1 이 이 계획의 전부다.** V2·V3 은 정리 작업이고, V1 은 tier-3 이 Phase-2/3+(OOP·CRV·SVA·
coverage·string·queue·real math·program·vif)를 따라잡는 일이라 **길다**. V0 이 그 길이를 숫자로
말해 줬다 — **14 슬라이스, 상위 셋이 87%.**



> **⭐⭐ S3 착수 (2026-08-10 · §4.5.333) — ③층이 컴파일된 바디를 실행한다.** `run_body`(SimIr 워크)
> 대신 `backend::vm_exec`(`CompiledBody`) — 재진술 0(`impl Kernel` 로 제네릭이라 같은 커널 메서드를
> 같은 순서로 부른다). **이것이 코드젠의 입력이다**(`jit.rs::compile_body(&CompiledBody)`).
> picorv32 워크 대비 **+3.9%**(native/vm 1.57→**1.64×**).
>
> ⭐⭐ **그리고 이 슬라이스의 진짜 산출은 판정이다: 컴파일된 표현 자체는 win 이 아니다.** 융합 전은
> 완전한 wash 였고 — `k_resolve_lvalue_offsets` 3.1% 가 사라진 만큼 op 루프와 `Value` 레지스터
> 왕복이 먹었다 — 평가+쓰기를 한 op 으로 융합(op 2000→1068·대입 93.7%)해야 +3.9% 가 나왔다.
> **남은 비용은 커널 호출을 *고르는* 데가 아니라 그 *안*에 있다**(`WProg::run` 20.4% ·
> `write_lvalue` 9.8% · `settle` 8.3%) → **S3 의 값은 전적으로 코드젠 단계(호출 자체를 없애는 것)에
> 있다.** ⭐⭐ 부수로 `vm_exec` 에 **문장 경계가 없던 것**(②층에도 있던 `call_fatal` 발산)을 닫았다.
>
> **남은 계획 = S3 코드젠 본체 → S4 스케줄 소거(<1.3× 면 유지) → S5 NBA 전용화 → S6 3-OS 결정성
> → 재측정(≥30× 못 내면 v1 실패로 기록).** S3 중단 판정 = **호출을 삼키지 못하면 중단.**


> **⭐⭐ S2 슬라이스 4 완료 (2026-08-07 · §4.5.312) — 런타임 배열 원소 읽기.** `wprog` 의
> `Expr::Signal` 팔이 워드 인덱스를 상수로만 받던 것을 **런타임 인덱스**까지 admit 한다
> (`WOp::LoadIdx` · `Value→워드 인덱스` 규칙은 `eval::word_index_of` 로 **추출 공유** — 그 규칙이
> E4002 를 소유한다). 런타임 인덱스 마이크로벤치 **4.013 s → 0.64 s**(native/vm 0.27× → **1.70×**),
> **picorv32 0.83× → 0.97×**. ⭐⭐ 라운드 1 이 내 수정 안에서 **진단 중복**을 잡았고(슬롯을 하나씩
> 컴파일하며 즉시 실행 → admit+OOB 뒤 decline 하면 제네릭 resolver 가 같은 접근을 재보고 → 8개 cap
> 잠식) 수정은 **결정과 실행의 분리**다. ⭐ 게이트는 **오프셋만** 비교하고 있었다 → 보고 수 비교 +
> decline 부작용 0 단언 + 판별 설계 3문장.
>
> **⭐⭐ S3a 완료 (2026-08-07 · §4.5.311) — 호출 흡수.** `NetArena::buildable` 의 blanket
> `func_table` 거부가 **측정된 부분집합**(`native::frames`)이 되고 `NativeKernel` 이 **복합
> `NetReader`** 로서 호출을 엔진 프레임 실행기에 위임한다(재진술 0). `bench/keccak` **호출형·
> 배열형이 네이티브·바이트 동일** → ②→③ 재측정이 세 변종 전부에서 가능.
>
> **⚠️ 다음 단계가 진짜 코드 생성이고 아직 0%다.** 지금의 `--backend native` 는 기계어를 안
> 만든다(전용 저장 + 폭 특수화 평가기 = 세 번째 인터프리터). 실측 native/vm = **1.46×**(flat) ·
> **1.15×**(호출형) · **1.06×**(배열형) · **0.81×**(picorv32) · verilator 대비 **54×~722×** 뒤짐.
>
> **⭐⭐ S2 1비트 연산자 슬라이스 완료 (2026-08-08 · §4.5.315).** 그 그라운딩이 지목한 표적을
> 그대로 쳤다 — 거절 ~300 중 **276**이 `&&`·`===`·`?:`·`||`·`|x`·`!` 여섯이었고, `?:` 를 뺀
> 다섯 + reduction 전부를 admit 했다. picorv32 admission **60.8% → 66.3%** · native
> **1.085 → 0.984 s(1.10×)** · native/vm **0.81× → 0.88×**. 의미는 재진술하지 않았다(자유 함수 7개
> 추출 + 메서드 위임). **⚠️ `?:`(34건)는 별개 슬라이스** — 제네릭이 취한 가지만 평가하므로
> `&&`/`||` 를 admit 시킨 "단락하지 않는다" 논거가 **전이되지 않는다**(untaken 가지의 `LoadIdx` 가
> 없던 E4002 를 만든다). 남은 거절 = `?:` 34 · `Select` 12 · 폭 불일치 14 · `Concat` 4 · `SysFunc` 3.
>
> **⭐⭐ 2026-08-08 그라운딩이 이 항목의 전제 셋을 정정했다**(상세·프로파일 표 = preview/21):
> ⓐ 여기 적혀 있던 picorv32 **0.97× 는 stale** 이고 현재 **0.81×** 다. ⓑ 그러나 **native 의 회귀가
> 아니다** — native 절대시간은 세 슬라이스 내내 평평하고(1.066→1.072 s) **VM 이 aes_top 에서 1.18×
> 빨라졌다**(1.035→0.878 s). *비율만 기록하면 이 축은 거짓말을 한다 — 절대시간을 함께 남길 것.*
> ⓒ **"picorv32 의 남은 병목은 스케줄러" 는 VM 쪽 성질이지 native 쪽이 아니다**: native 프로파일
> 1~6위가 제네릭 트리워커(`eval_ctx` 15.8% · `Value::mask_top` 11.2% · `Value::resize` 10.4% ·
> `truthiness` 6.7% · `read_net` 6.0% · `eval_binary_ctx` 5.4% ≈ **50%**)이고 S2 가 지은
> `run_wprog` 은 **4.0%** 뿐이다(VM 쪽이 `propagate_changes` 40.1%). 즉 picorv32 의 식은 대부분
> **admit 되지 않는다** — `wprog::compile_node` 의 진입 조건이 서브트리 **폭·부호 균일**이라
> 32비트 데이터패스에 5비트 select·1비트 flag·concat 이 섞이면 트리 전체가 거절된다.
> **⇒ 순서는 admission 확대(S2 본체) → S3 다.** 커밋된 대조군이 그것을 재현한다: 균일 64비트인
> `keccak_f_flat` 은 같은 프로파일러에서 `run_wprog` 이 **18.4% 로 1위**이고 `eval_ctx` 는 상위 8위
> 밖이다. S3a 는 **행을 열었을 뿐 호출 비용은 안 샀다** — flat↔호출형 **13×** 는 여전히 S3 의 표적이고,
> keccak 은 이미 균일하므로 S3 의 이득은 그쪽에서 먼저 재는 것이 맞다.
>
> **S3b(잔여)** = 프레임이 모듈 넷을 읽게 — `(st, nets)` 헬퍼에 split 리더 배선 →
> `k_dispatch_systask` · delayed CA · task 프레임. 거부 사유는 `native::frames` 가 각자 이름으로 말한다.

**판정이 뒤집혔다.** 정본·근거·파괴 범위 = [preview/21 §0.3 + §7](preview/21-tier3-native-backend.md).

> **S1d-4d-3 완료 (2026-08-05 · §4.5.302) — delayed CA.** `assign #d` 배선 → **코퍼스 72/72 네이티브·
> 바이트 동일**(30→65→72). 재진술 대신 **추출**(엔진도 같은 함수를 통과·PRE/POST 179설계 0 diff).
> ⭐⭐ 그런데 추출이 절반이었다 — **LHS 오프셋만 엔진 스토어**로 해석해 동적 인덱스 쓰기가 조용히
> 사라졌다(10설계). ⭐⭐ 그리고 공유로 옮긴 부분은 **차분이 원리적으로 못 지킨다**(generation 필터·
> `transition_delay` 를 지워도 전 게이트 통과) → **iverilog 절대값 앵커 2개** 신설.
>
> **S1d-4d-2 완료 (2026-08-05 · §4.5.301) — VCD.** `$dumpfile`/`$dumpvars` 배선 → **`examples/` 넷
> 전부 네이티브·stdout+VCD 바이트 동일**(원래 S1 게이트가 실사용 설계에서 통과). 코퍼스 스트립 폐지
> (dump 44 중 37 이 VCD 까지 비교). ⭐⭐ differential 이 **내 주석의 주장을 반증**(`arg_string` 은
> early-return 하지 않는다 → `$dumpfile(nm)` 이 `x` 파일에 썼다·이름만 다르다) · ⭐⭐ soundness 가
> 게이트의 눈먼 축 **다섯**을 셌다(폭>64·런중 x/z·두 드레인 사이 두 쓰기·`$dumpoff`) · ⭐ 공유 코드
> 뮤테이션은 **원리적으로** 이 차분이 못 잡는다(양쪽이 같이 움직인다).
>
> **S1d-4d-1 완료 (2026-08-05 · §4.5.300) — zero-delay cont-assign settle.** 거부가 blanket 에서
> **delayed·wired·multi-driver** 셋으로 좁아졌고 **picorv32 가 네이티브로 돌며 바이트 일치**한다.
> ⭐⭐ 내 byte-identity 논증이 **값 축에서만** 참이었다(워크리스트 없이 매 패스 방문 = 재평가마다
> `E4002` 재발화 · picorv32 6→9) · ⭐⭐ 두 렌즈가 **`arm_t0` 가 t0 settle 의 변경 집합을 버린다**로
> 수렴(퍼즈 270 중 49 발산) · ⭐⭐ **게이트 이빨 0**(settle 최상단 panic 이 전 스위트 통과 — 신호는
> `ran` 이 65 에서 안 움직인 것이었다).
>
> **S1d-4c-2d 완료 (2026-08-04 · §4.5.299) — in-body 웨이터.** `Wait{Edge|Level|Expr}` 를 ③층이
> 실행한다(`k_suspend_on` · 공유 워크의 `Wait` 암 · `fire_waiters`). ⭐ 그라운딩이 큐를 정정: **`Named`
> 는 구성 불가**(named event → 카운터 넷 → `Level`). ⭐⭐ 두 렌즈가 같은 silent-wrong 으로 수렴 —
> `fire_waiters` 가 범위 진단의 **세 번째 생산자**인데 `propagate` 뒤에 드레인이 없어 세 종료 경로
> 전부에서 사라졌다(**FAIL→PASS**). ⭐⭐ 그리고 내가 코드에 적은 *"S0 가 먼저 거부한다"* 가 틀렸다 —
> **`wait fork;` 는 `fork_modes` 를 안 만든다**(eligible ∧ buildable, 이 암이 유일한 거부자).
>
> **S1d-4c-2c 완료 (2026-08-04 · §4.5.298) — ③층이 처음으로 설계를 돌린다.** 런 루프(리전 큐·델타
> 루프·시간 진행·`Delay` 정지·`busy`) + `simulate` 배선 + **세 번째 게이트 층**
> `native::run::executor_rows`. 슬라이스 경계는 측정이 정했다 — 코퍼스 **72 중 0** 이 전 프로세스
> 정지-없음이고 정지 터미네이터 **138 개가 전부 `Delay`** 라, 오라클이 있는 단위는 "루프 + `Delay`"
> 이고 in-body 웨이터는 다음이다. ⭐⭐ 두 적대 렌즈가 **같은 loud→silent 로 수렴**: OOB 배열 인덱스의
> `warn_run_range` 가 아레나에 없어 평범한 FIFO 가 `FAIL` → `PASS`(stdout 은 바이트 동일) — 트리가
> 이미 적어 뒀으나 "stderr 문제"로 **크기를 잘못 쟀다**(실제로는 exit class). ⭐ 뮤테이션 26 중 12
> 생존 → 판별 설계 9개로 **2 로**(남은 둘은 등가). ⚠️ **`examples/` 4개와 `bench/` 둘은 전부 거부**
> 된다 — 65/72 는 루프의 커버리지이지 실사용 설계의 커버리지가 아니다.
>
> **T0+S0 완료 (2026-08-03 · §4.5.285)** — run.json `codegen`/`native` 계기 + 설계 수준 게이트.
> **적격률 79/79 → S0 중단 판정 통과.**
>
> **S1d-3 완료 (2026-08-03 · §4.5.290)** — **wake 결정**(변경 집합 → ready 집합과 순서). ⭐ 적대
> 리뷰가 **조합 프로세스 전체가 미등록**임을 잡았다(`arm_sensitivity` 는 `Level|Comb|Latch` 셋 다
> 같은 웨이터로 만든다) — 그리고 **등록만 고치면 게이트가 깨진다**(t0 arm 상태가 짝). 8규칙 teeth.
>
> **S1d-2 완료 (2026-08-03 · §4.5.289)** — **dirty/edge 채널**(`dirty` 멤버십 = 변경 집합 · 
> `last_blocking_writer` · **`slot_edge`** = 끝점이 잃은 엣지 **종류**). edge-target 스캔은 엔진과
> **한 철자**. ⭐ 게이트 teeth 를 세 번 재확인했고, 적대 리뷰가 **두 store 지점 중 bit-serial 쪽이
> 0회 진입**(그 블록을 지워도 전 패키지 초록)임을 실측해 닫았다.
>
> **S1d-1 완료 (2026-08-03 · §4.5.288)** — S1d 를 넷으로 분해하고 첫 조각: `Backend::Native`·
> `--backend native`·`native::runtime_gate`(설계 게이트 ∧ 아레나 빌드). 실행기가 없어 **항상 VM 폴백**,
> run.json 이 **실제 실행기**와 **두 층 판정**(`eligible`/`buildable`/`refused`)을 싣는다 — 79/79 vs
> **78/79** 격차가 이제 기계로 읽힌다. S1c 가 남긴 필수 4건 중 2건 종료.
>
> **S1c 완료 (2026-08-03 · §4.5.287)** — 아레나 쓰기 퍼널(엔진 write 사슬의 Value-수준 미러) +
> 오프셋 해석기 단일화(`eval::resolve_offsets`) + 게이트 문장 스캔 2행(force/release·disable).
> ⭐ 문장 단위 차분이 **내 퍼널의 silent-wrong 1건**을 잡았다(real 값 rhs + real 넷 없음 = 적격인데
> 엔진은 반올림·아레나는 IEEE 비트). 적격률 79/79 불변. **S1d 착수 前 필수 3건**은 §1 NEXT 0번.
>
> **S1a+S1b 완료 (2026-08-03 · §4.5.286)** — S1 을 4 내부 단계로 분해(정본 = preview/21 §5 S1
> 분해 표). `NetArena`(R1 저장 소유·슬롯 desc 가 per-access 질문 전부 대체) + `impl NetReader
> for NetArena`(기존 eval 밑 = 평가 의미 공유). **R1 중단 판정 통과**(corpus 72 폭별 슬롯 전수)
> · 게이트 = init parity 297넷 + **미러 차분 17,940건 발산 0**. 다음 = **S1c 쓰기 퍼널**
> (bit/part/array lvalue·OOB drop·X-index no-op·NBA 샘플 — 게이트 = 한 문장 실행 후 양 스토어
> 값 동일), 그 다음 **S1d 자기 스케줄러 + `--backend native` 배선**(corpus stdout+VCD 바이트 동일).

| | |
|---|---|
| 실사용 격차 | **≈108×** (리포터 작업량 기준 정정치 — 레코드 수로 나눈 ~200× 는 과대평가) |
| ②층 전부 청구 시 | **~11×** (T1~T4 는 같은 경로라 곱하지 않는다) |
| 남는 것 | **≈10× — ②층으로 도달 불가** |

T1/T2 가 푸는 문제("호출을 가진 바디를 컴파일 대상으로")는 ③층이 **폴백 없이 다시** 풀어야 한다
(§4.1). 먼저 하면 같은 설계를 두 번 하고, ②층 버전은 ③층 완성 시 죽은 코드가 된다.
→ **T0 만 유지**(계기·되돌리기 0·S0 이 같은 자료를 쓴다), **T1~T3 은 ③층에 흡수**,
**T4** 는 ③층과 무관한 국소 결함이라 **기회 슬라이스**로 독립 유지.

단계별 착수 게이트와 중단 판정은 preview/21 §7.3 이 정본. 성공 기준 = 리포터 워크로드
**≥30×**(56 분 → 2 분).

### 5.0-b (참고) T 단계의 원래 측정치 — ③층 예산의 근거 (§4.5.282)

정본 계획·중단 판정·근거표 = **[preview/21 §7.3](preview/21-tier3-native-backend.md)**.
측정 원본 = **[preview/18 round-26](preview/18-acceleration-analysis.md)**. 요약:

| | 서브루틴 호출 있음 | 인라인 | |
|---|---|---|---|
| vita | 5340 µs/순열 | **498 µs** | ← **10.7× 차이, 차이는 함수 호출뿐** |
| iverilog 13 | 4450 µs | 1398 µs | |
| verilator (③층) | 6.6 µs | 6.56 µs | ← ②→③ = **76×** |

| 단계 | 무엇 | 측정 목표 | 중단 판정 |
|---|---|---|---|
| **T0** | VM 커버리지 계기화 — 설계별 codegen 적격률 + **거부 사유 히스토그램**을 `--obs-dir` run.json 에. 현재 유일한 관측 수단이 `--backend` A/B 다 | 설계 4종 적격률·사유 상위 3 | — (계기) |
| **T1** | `Terminator::Call` 을 가진 프로세스를 VM 이 받는다(호출만 기존 프레임 실행기로 콜아웃) | Keccak 호출형 **≥1.5×** | <1.2× → 중단, T2 로 |
| **T2** | 프레임 바디를 VM 대상으로 — `codegen_coverage` 를 `func_blocks` 까지. 상한은 이미 실측(2.1×) | 인라인형 대비 격차 **절반 이하** | <1.3× → 중단 |
| **T3** | 프레임 호출 단가 **650 ns**(iverilog 375). 프로파일 1위가 `Value::resize`/`mask_top`/`clone` ⇒ 인자·반환 Value 왕복이 표적 | **≤300 ns** | 500 ns 못 내리면 기록 후 중단 |
| **T4** | 함수 지역 배열 원소 쓰기 **514 ns**(iverilog **24 ns**, 21×). 모듈 배열은 대등 ⇒ 국소 결함 | iverilog **2배 이내** | 5배 이내 실패 시 기록 후 중단 |
| — | **재측정 게이트** — Keccak·PicoRV32·리포터 워크로드 3종. 그때의 ②→③ 격차가 ③층 예산 | — | — |

> **T 는 ③층을 미루는 것이 아니라 ③층 예산을 확정한다.** 그리고 ③층 백엔드도 **같은 호출 문제**를
> 풀어야 하므로(호출을 못 삼키는 ③층은 똑같이 커버리지 0%), T1/T2 의 콜아웃 ABI 설계가
> doc-21 §4.1(설계 단위 all-or-nothing)의 **리허설**이다.

### 5.1 나머지 (전부 보류 판정 — 트리거 시만)

- **CI 를 `cargo nextest` 로 옮긴다**(2026-08-06 실측·오너 지시로 별도 슬라이스): `cargo test --workspace` 는 450 타깃을 **순차** 실행해 실행 단계만 **724 s** 인데(per-target 시간 합은 62 s — 나머지는 프로세스 기동), `cargo nextest run --workspace` 는 같은 5183 테스트를 **30 s** 에 돌린다(24×). 로컬은 이미 nextest 로 통일했고 CI 4잡(ci.yml)만 남았다. ⚠️ 두 실행기는 **빌드 트리를 공유하지 않으므로** 번갈아 쓰면 전환마다 ≈470 s 재빌드를 문다 — CI 도 옮겨야 로컬/CI 가 한 실행기가 된다. ⚠️ 버전은 **0.9.100** 을 쓴다(0.9.143 은 rustc 1.91 요구·저장소 핀은 1.85). 슬라이스 내용 = ci.yml 4잡 교체 + nextest 설치 단계 + `--locked` 유지 + 3-OS 에서 결과 동일 확인. ⚠️ **선결 조건 — temp 디렉터리 이름이 nextest 아래에서 충돌한다**(2026-08-07 실측): 통합 테스트 **368개 파일**이 `temp_dir()/vita_<tag>_<pid>_<프로세스별 카운터>` 를 쓰는데, nextest 는 **테스트마다 프로세스를 새로 띄우므로** 카운터가 매번 0 부터 시작하고 **PID 재사용**이 이전 실행의 디렉터리와 겹친다 — `cli::obs::compile_error_writes_no_obs` 가 그렇게 한 번 빨갛게 났고(격리 실행·연속 두 번의 전체 실행은 초록) `cargo test`(한 프로세스)에서는 구조적으로 안 난다. 이름에 프로세스 고유값을 더하거나 생성 전에 기존 디렉터리를 지워야 CI 를 옮길 수 있다.
- **"새 Rust 를 지원한다" 는 정책이 검증되지 않는다**(2026-08-06 실측): 툴체인 정책은 *"MSRV 에 상한을 두지 않는다 — 새 Rust 가 나오면 지원하는 쪽으로 따라간다"* 인데, `rust-toolchain.toml`·`Cargo.toml rust-version`·ci.yml **4잡 전부 1.85.0 고정**이고 `stable`/`beta` 잡이 **0개**다. 즉 바닥은 강제되지만 **천장은 아무도 안 밟는다** — vita 가 1.91/최신에서 빌드되는지 확인하는 곳이 없다. 1.85 자체는 의도된 값이다(§4.5.150 에서 fst-writer 0.3.x 의 edition 2024 때문에 1.82→1.85·vita 크레이트는 edition 2021 유지·저장소 안에 1.85 초과를 요구하는 것 없음·PUBLIC 이라 MSRV 상향은 사용자 약속을 좁힌다). 필요한 것은 상향이 아니라 **비차단 `stable` 잡 1개**.

- **`native::run::executor_rows` 가 모든 `simulate` 호출에서 돈다**(§4.5.298 리뷰): 백엔드와 무관하게
  전 프로세스의 전 문장을 훑는다. 요청 백엔드로 가드하면 run.json 의 `native` 판정이 요청에 따라
  달라져 "census 는 실행기와 무관" 계약이 깨지므로 **무조건이 맞다** — 비용이 문제가 되면 캐시.


- **u32::MAX 를 넘는 `#delay` 는 여전히 CLAMP(§4.5.292 잔여)**: 랩(7× 조기 발화)은 고쳤지만
  표현 불가 값은 조용히 `u32::MAX` 로 잘린다. 표현하려면 IR 필드(u32)를 바꿔야 하고(동결 타입 →
  format bump), 알리려면 **새 W-code** 가 필요하다. 지금은 4.29e9 틱에 실제로 도달하는 런에서만
  틀리다(전에는 그런 delay 전부가 틀렸다). 이득 = correct-or-loud 완성.
- **③층 판정에 "오늘의 커널이 돌릴 수 있는가" 층이 없다**(§4.5.292): run.json 은 `eligible`(범위)
  과 `buildable`(오늘의 저장소)만 싣는데, `$sformatf`·`$display`·transport-delay NBA·재arm 은
  **적격이고 빌드되지만 커널이 아직 없다**. `kpred::rhs_routes_to_worker` 가 그 질문의 절반을
  이미 답하지만 게이트에 안 물려 있다 — 4b 가 dispatch 를 배선할 때 **세 번째 층**으로 실어야
  그 전까지 "적격"이 능력으로 오독되지 않는다.
- **③층 quiescence 가 커널의 `delayed_nba` 를 안 본다**(§4.5.295·**S1d-4c-2 필수**): 엔진의 `next`
  는 `Scheduler` 의 `wheel`/`delayed_ca`/`delayed_nba` 최소값인데 네이티브 런에서 그 맵은 비어 있다
  (모든 `k_schedule_nba*` 가 커널 큐로 간다) → **트랜스포트가 유일한 대기 작업이면 quiescent 로
  보고되고 업데이트가 사라진다**. `k_schedule_nba_at` 이 loud 패닉에서 조용한 enqueue 로 내려온
  상태라 이건 **사다리 하강의 유일한 잔여**다.
- **S1d-4d 바이트 동일 게이트가 만날 pre-existing 오라클 차이 6건**(§4.5.295 differential 실측·전부
  PRE==POST): ① `$finish` 와 같은 틱의 pending NBA/트랜스포트를 iverilog 는 적용+덤프하는데 vita 는
  안 한다(**파형만**·stdout 은 일치) ② VCD intra-tick 입도 — vita 는 NBA store 마다, iverilog 는
  정착값만(`native/dirty.rs` 의 의도된 설계) ③ t=0 initial 블록 순서(소스 순서 의존·LRM 미정의)로
  `@(negedge clk)` 가 iverilog t=2 · vita t=0 ④ **t0 arm 순서** — vita 는 `initial` 이 t=0 에 쓴
  x→0/x→1 전이로 static edge 를 발화시키는데 iverilog 는 안 한다(리셋 펄스를 t≥1 로 옮기면 일치) ⑤
  **`always @(*)` 의 읽기 집합이 비면** vita 는 t0 에 한 번 돌고 iverilog 는 안 돈다 ⑥ **`.velab` 은
  별개 `vcmp` 실행 간 바이트 재현이 안 된다**(RULEV-MTIME `(mtime,size)` 스탬프 8바이트 — 의도된
  설계지만 **naive staged 아티팩트 바이트 게이트는 flaky** 해진다 · work lib 을 고정하면 재현됨).
  **게이트를 짜기 전에 여섯 개를 어느 쪽으로 고정할지 정해야 한다.**
- **`$monitor`/`$strobe` 의 ③층 렌더 경로**(§4.5.294 거부): dispatch 는 등록만 하고 렌더는
  `sched/run_loop.rs::flush_postponed` 에서 일어나는데 그 경로는 리더를 안 받는다. 배선하면 거부를 풀
  수 있다(S1d-4c 와 한 슬라이스 — 그 경로가 곧 리전/포스트포운드 큐다).
- **`NetArena` 의 `fd_eof` X-poison 구멍**(§4.5.293): 아레나의 defaulted `NetReader` 메서드 중
  `fd_eof` 만 "heap/class/frame 없음" 논증이 안 덮는다 — 지금은 **`$feof` 과잉표시가 가리고 있을 뿐**이라
  그 과잉표시를 고치면 `$display("%0d", $feof(fd))` 가 적격이 되고 아레나는 X 를, 엔진은 실제 플래그를
  낸다. 아래 `$feof` 항목과 **한 슬라이스로 묶어야 한다**.
- **`$feof` 가 정본 stmt-effect 술어에서 과잉표시**(§4.5.291 적대 리뷰 실측): `k_feof` 는
  `read_state[fd].eof` **순수 읽기**이고 elaborate 주석도 그렇게 적는데 `sysfunc_is_stmt_effect` 가
  `true` 로 표시한다 → `e = $feof(fd);` 는 ③층 게이트가 거부하고 `while (!$feof(fd))` 는 통과한다.
  술어를 한 소비자에서만 고치면 **철자가 둘**이 되므로(두 백엔드가 갈릴 수 있다) 정본을 고쳐야 하고,
  그것은 **tier-2 의 컴파일 게이트도 함께 넓히는** 변경이라 자체 byte-identity 논증이 필요한 별도
  슬라이스다. 이득 = ③층 과잉거부 1형 해소 + tier-2 가 그 바디를 컴파일 대상으로.
- **`NetSlot.prev` 는 워크스페이스 전체에서 읽는 곳이 0** (§4.5.290 적대 리뷰가 필드명을 바꿔 빌드해 증명 — 선언·생성자·`propagate_changes` pass (c) 쓰기 세 곳만 걸린다). 즉 pass (c) 의 `clone_from` 2회/변경넷/델타가 **핫 루프의 순수 죽은 일**이다. 제거는 바이트 동일이 자명하나(아무도 안 읽음) 그 자명함 자체를 검증해야 하므로 **별도 슬라이스**로 둔다 — 값은 perf 축, 위험은 "정말 아무도 안 읽는가" 하나.

- **✅ COMB-DEPTH 해결 (2026-08-01) — dirty-settle. 깊이 24 에서 14.1× · 출력 바이트 동일 · 골든 재판정 0건.** `settle_cont_assigns` 가 매 델타 전체 cont-assign 을 전수 재평가하던 것을 **의존성이 움직인 것만** 재평가하도록 바꿨다(`propagate_changes` 를 305→15.5 ms 로 만든 dirty-list 와 같은 형태). 인스턴스 체인, 총 사이클 고정:

  | depth | before | after | 배속 |
  |---|---|---|---|
  | 1 | 7.8 ms | 5.7 ms | 1.4× |
  | 6 | 71.2 ms | 13.3 ms | **5.4×** |
  | 12 | 229.6 ms | 26.1 ms | **8.8×** |
  | 24 | 814.4 ms | 57.9 ms | **14.1×** |

  **2차 항이 사라졌다** — 24× 깊이에 10.2× 로, cont-assign 없는 순수 체인(5.3×)과 같은 선형 형상. 검증 = 5016 tests green · examples 4종 **stdout+VCD 바이트 동일**(PRE `df3d8df` vs POST) · depth-6 체인 iverilog 차분 일치(`acc=00001f68` 3자 동일) · clippy/fmt clean · `SimIr` 무변경.

  **건전성 논거**: 의존성이 안 움직인 assign 은 같은 값을 다시 계산하고 write 퍼널이 same-value write 를 변경 없이 버리므로, 건너뛴 방문은 **관측상 no-op** 였다. 그래서 모든 위험이 **의존성 집합의 완전성**에 있고, 그것을 `levelize::ca_deps` 가 positive allow-list 로 인증한다 — delayed · multi-driver 멤버 · 비순수 RHS(`SysFunc`/`Call`/`ArrayItem`) · **heap handle 의존**(dyn/queue/assoc/class: 핸들 net 이 안 바뀌어도 내용이 바뀔 수 있어 `note_change` 가 보고하지 않는다)은 전부 **무조건 재평가** 목록으로. 방문 순서는 오름차순 = **선언 순서**(기존 fixpoint 와 동일, 골든 다수가 의존).

  ⭐ **구현 중 발굴**: `Expr::Signal { word }` 이 상수 인덱스가 아니라 **ExprId** 였다(`eval_core` 가 평가한다). `expr_nets` 가 그걸 안 걷고 있었고, 랭크에선 무해했지만 **dirty-settle 에선 `assign y = mem[idx]` 가 `idx` 변경에 재평가되지 않는 silent-wrong** 이 됐을 자리다 — 배선 전에 잡아 수정.
- **🔴 COMB-DEPTH 중간 판정 (2026-08-01, 위에서 해결됨) — 원인이 §4.5.278 이 지목한 자리가 아니었다. levelize 는 불필요하고, 오너가 승인한 정확성 대가도 불필요하다.** 오너가 재진입 트리거 ①②를 둘 다 승인해 D(levelize)에 착수 → **랭크(`sim_engine::comb_ranks`)를 만들고 랭크 순 Active 드레인 + 랭크 사이 settle 을 구현해 측정했더니 깊이 1~24 전 구간에서 1.00×** → **폐기·되돌림**(아무것도 안 하는 knob 을 남기지 않는다 = §4.5.278 `Value::resize` 판례). 총 사이클 고정·깊이만 스윕(`perf_depth_cost_shape`):

  | depth | 단일 모듈(cont-assign 없음) | 인스턴스 체인(포트 cont-assign) |
  |---|---|---|
  | 1 | 3.3 ms | 7.8 ms |
  | 6 | 8.4 ms | 71.2 ms |
  | 12 | 15.3 ms | 229.6 ms |
  | 24 | 31.7 ms | **814.4 ms** |

  **순수 `always_comb` 체인은 깊이에 선형**(24단이 1단의 9.6× = 깊이 페널티 0)이고 wake 사슬이 델타당 프로세스 1개라 **애초에 정렬할 배치가 없다.** 2차 비용은 **cont-assign 을 거칠 때만** 나타난다(24× 깊이에 104×). 뿌리 = `settle_cont_assigns` 가 **매 델타마다 전체 cont-assign 을 fixpoint 까지 전수 재평가**하고, 깊이 D 체인은 전파에 D 델타를 쓰며 O(D) 개의 assign 을 들고 있다 → O(D²). **프로세스 드레인 순서로는 손댈 수 없다.** 레버 = **dirty 기반 settle**(RHS 넷이 바뀐 assign 만 재평가) — `propagate_changes` 를 305→15.5 ms 로 만든 dirty-list 와 같은 형태. **결정적으로 이건 프로세스 실행 순서를 안 바꾸므로 골든 재판정이 전혀 필요 없다** — 오너가 승인한 대가를 치르지 않고 얻는다. 잔존물 = `comb_ranks`(조합 깊이 보고용, 정확·저비용). 다음 수 = **dirty-settle**. 상세 = `crates/sim-engine/src/levelize.rs` 모듈 독 · 구 판정 아래.
- **COMB-DEPTH(외부 round-23 §3.2) — 구 판정(2026-07-31): vita 결함 아님, 재진입 트리거만 등재.** 총 작업량을 고정한 깊이 스윕에서 **iverilog 가 같은(더 가파른) 스케일링**을 보였다(UNROLL 1→12 에서 iverilog 4.15× vs vita 3.55×, 절대 wall 은 vita 가 0.80–0.99×). 깊이 비용은 인터프리티드 이벤트구동 델타사이클의 성질이고, 리포터의 비교 대상(Xcelium, <10분 vs vita ~13시간 추정)은 **컴파일-타임에 levelize 하는 컴파일드 시뮬레이터**다. 원인은 특정됨: UNROLL=6 에서 사이클당 프로세스 활성이 `7 6 5 4 3 2 1…` 삼각형(≈D²/2)이고, 단 사이 전파가 cont-assign settle 즉 **델타 경계**를 거치므로 **배치 정렬은 지렛대가 아니다**(오름/내림 정렬 4.86/4.85 s 동일 — 실측으로 기각). 승격하려면 **rank 순 active 배출 + rank 사이 settle**(levelize)이 필요하고, 이득 상한은 ≈D/2(UNROLL=6 에서 ~3×)인데 **프로세스 실행 순서를 바꾸므로 다중 프로세스 `$display` 순서가 이동**한다(IEEE §4 는 region 내 순서를 implementation-defined 로 두지만, vita 의 골든은 iverilog 순서에 핀되어 있다). 재진입 트리거 = ①깊은 조합 cone 실수요 + ②순서 이동을 감수한다는 오너 판정. 상수항은 별도 축이다: trivial flop 200k 사이클에서 vita 1.03M cyc/s vs iverilog 1.96M cyc/s(**1.91×**) — `Value::resize`/`mask_top` 원워드 fast-path 는 **측정 이득 0** 으로 기각(§4.5.278). 상세=ARCHIVE §4.5.278 · 가속 분석 = [preview/18](preview/18-acceleration-analysis.md).
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

> **✅ E-SPIKE (2026-08-01) — 프로세스 융합이 컴파일드 전환의 enabler 임이 실측 확정.** C-GAIN 곡선이 "VM 수익 = 활성당 작업량의 함수"(1문장 0.99× · 64문장 0.75×)임을 보였으므로, 남은 질문은 **무엇이 활성당 작업량을 올리는가**였다. 답 = **프로세스 융합**. 동일 로직·동일 깊이·동일 사이클을 세 형태로 재비교(`perf_fusion_spike`):
>
> | depth | form | interp | vm | vm/interp | VM 배속 |
> |---|---|---|---|---|---|
> | 24 | instances(포트 cont-assign) | 59.7 ms | 46.8 | 0.78× | 1.28× |
> | 24 | separate(모듈 내 `always_comb` 24개) | 31.8 ms | 20.4 | 0.64× | 1.56× |
> | 24 | **fused(`always_comb` 1개·24문장)** | **21.0 ms** | **9.4** | **0.45×** | **2.22×** |
> | 48 | instances | 143.4 ms | 120.4 | 0.84× | 1.19× |
> | 48 | separate | 73.2 ms | 49.0 | 0.67× | 1.49× |
> | 48 | **fused** | **39.8 ms** | **16.5** | **0.41×** | **2.44×** |
>
> **융합은 두 축을 동시에 민다** — ①인터프리터 자체가 143.4→39.8 ms(**3.6×**, 프로세스 수·델타 수·활성당 오버헤드 감소) ②그 위에서 VM 수익이 1.19×→**2.44×**. 합산 **8.7×**(instances+interp 143.4 → fused+VM 16.5). 곡선은 depth 48 에서도 **포화 전**(0.45→0.41)이라 JIT 여지가 그 위에 남는다. ⇒ **F 기각 판정(잔여 1.26–1.50×)은 융합 전 바디 크기에서 잰 값이므로 ② 이후 재측정 대상.** 등가성: `fused == separate` 가 전 깊이 일치(`instances` 는 stage 모듈이 매 단 `+1`, 나머지는 `+i+1` 이라 설계상 다른 함수 — 값 차이는 의도된 것).
>
> **✅ E-OPP (2026-08-01) — 융합 기회 실측. 실 RTL 형태에는 있고, 인접-프로세스 융합만으로는 못 잡는다.** 기계를 짓기 전에 기회를 재는 규율(§C/§F 를 살린 것과 동일):
>
> | 설계 | 프로세스 | 인접 융합 | 사슬 | **복사-가로지르기** |
> |---|---|---|---|---|
> | inst-chain d=24 (실 RTL 형태) | 26 | **0** | 0 | **23** |
> | separate d=24 (모듈 내 체인) | 26 | 23 | 24 | 23 |
> | sha256 · expr-heavy · examples 4종 | 2–5 | 0 | 0 | 0 |
>
> **인접 융합은 실물에서 0 이다** — 실 RTL 의 단 사이는 `always_comb r` → `assign y = r` → 포트 → 다음 단이라 **중간에 복사 cont-assign 사슬이 낀다**(포트 연결 하나가 복사 1개가 아니라 **사슬**로 내려간다 — 복사 1개만 가로지르는 1차 구현은 여전히 0 이었고, 사슬 끝까지 따라가서야 23 이 나왔다). ⇒ **② 의 유효 형태 = 복사-사슬을 가로지르는 융합**, 인접 융합이 아니다. examples 가 0 인 것은 제약이 아니라 그 설계들이 2–5 프로세스라 **조합 사슬 자체가 없어서**이고, 그런 설계엔 깊이 비용도 없다 — **기회는 비용이 있는 곳에 정확히 있다.**
>
> **② 잔여 공수(복사-사슬 융합)**: 바디 병합(블록 재번호·`Goto` 타겟 오프셋) · 융합된 복사 assign 을 settle 에서 제외 · 활성/wake 부기(융합된 producer 가 독립적으로 깨지 않게) · 출력 바이트 동일 게이트. **엔진 init out-of-band** 라 `SimIr`/format_version 무영향.
>
> **다음 = ② 선택적 프로세스 융합(복사-사슬 형태).** 안전 조건(순서 이동 회피) = P·Q 둘 다 Comb · P 가 net n 을 blocking 쓰기 · **Q 가 n 의 유일한 level 독자** · n 의 blocking writer 가 P 뿐 · P 가 n 외에 쓰지 않음. 이 조건이면 P·Q 사이 델타에 끼어들 관찰자가 없어 융합이 순서를 안 옮긴다. 변환 위치 = **엔진 init(out-of-band)** — `SimIr` 무변경이라 골든/format_version 무영향. G2 제약: `--probe`/계층 VCD/`%m`/계층 참조가 지목하는 net 은 보존(융합해도 중간 net 쓰기는 write 퍼널을 그대로 타므로 VCD 기록은 유지된다).

> **🔴 ② 융합 REVERTED (2026-08-01) — 만들고, 재고, 게이트까지 걸고, 그래도 틀렸다.** 아래 ②/③/④ 기록은 유효하나 **융합 구현은 되돌렸다.** 이유:
>
> | stimulus | iverilog | vita nofuse | vita **fuse** |
> |---|---|---|---|
> | `#1 clk=1; #1 clk=0` (코퍼스 형태) | `00000288` | `00000288` ✓ | `00000288` ✓ |
> | **`clk=~clk; #1`** (init 과 같은 활성) | `xxxxxxxx` | `xxxxxxxx` ✓ | **`0000017c`** ✗ |
>
> **뿌리**: unfused 는 깊이 D 체인이 **D 델타에 걸쳐** 전파되므로, 같은 배치에서 깨어나 체인 **출력**을 읽는 프로세스는 *부분 전파된* 값을 샘플링한다. 융합하면 한 활성에 완주해 그 독자가 *완전 전파된* 값을 본다 → **exit 0·진단 없음·값이 다름 = silent-wrong**.
>
> 구현한 안전 조건은 체인의 **중간 net**(아무도 안 읽음)을 지켰지만 **출력이 언제 fresh 해지는가**를 지키지 않았다. 그런데 그 출력의 독자가 바로 flop — 조합 cone 의 존재 이유다. "출력에 동시 독자 없음"을 요구하면 안전 집합이 빈다 ⇒ **intra-delta 순서를 iverilog 에 핀한 시뮬레이터에서 융합은 의미보존이 아니다.**
>
> ⭐ **게이트가 왜 못 잡았나**: 코퍼스 전 설계가 `#1 clk=1` 형태(초기화와 첫 엣지가 다른 타임스텝)였다. **게이트는 그 안에 든 형태만큼만 강하다.** 반례를 `a_comb_chain_output_is_sampled_mid_propagation` 으로 영구 핀(양 백엔드) — 누가 융합을 다시 지으면 즉시 붉게 뜬다. 코퍼스 체인 템플릿은 유지(독립적 커버리지 개선).
>
> **오너가 승인한 ②(순서 이동·골든 재판정)로도 이건 못 산다** — `$display` 순서가 아니라 **오라클 대비 값 발산**이고, IEEE 는 intra-delta 순서를 implementation-defined 로 두므로 둘 다 합법이지만 vita 의 차분 방법론 전체가 iverilog 일치에 서 있다. 사다리 규칙상 silent-wrong 생성기는 기본 off 여도 실을 수 없다.
>
> **남는 것**: E 축은 닫힌다. 융합 없이는 ③의 f 상승(33→71%)도 없으므로 ④(F 기각)는 **더 강하게** 유지된다.

> **(기록) ②/③/④ 측정 (2026-08-01) — 융합 랜딩 · F 최종 기각 · 다음 레버는 넷 개수다.**
>
> **② 프로세스 융합 랜딩**(`SimOpts.fuse`, 기본 off). 인접 융합(②a)은 실물에서 0 이라 **포트-연결 복사 사슬을 가로지르는 형태**(②b)까지 구현. 전 지점 **출력 바이트 동일**:
>
> | shape | d | interp off→on | VM off→on |
> |---|---|---|---|
> | in-module | 48 | 73.1 → 42.2 (1.73×) | 49.4 → 19.8 (**2.50×**) |
> | instances | 48 | 145.5 → 73.0 (1.99×) | 121.2 → 51.8 (**2.34×**) |
>
> 게이트 = `fused_equals_unfused_over_corpus`(72 설계 × off/on × 양 백엔드: stdout·VCD·summary) + 공허성 teeth + **`fused_instance_chain_matches_iverilog`**(외부 오라클 핀 `acc=0000340c`, iverilog 13 실측). 디버그로 잡은 것 2건: prelude 멤버의 `rearm` 이 사이클마다 Level waiter 를 누수시켜 **2차 슬로다운**(2.1→378.2 ms) · 등가 게이트가 **공허하게 통과**(코퍼스에 융합 대상 0 → 체인 템플릿 추가).
>
> **③ Amdahl 천장 재측정** — 융합이 f 를 두 배 이상 올린다:
>
> | shape | fuse | total | f | 상한 |
> |---|---|---|---|---|
> | in-module 48 | off → **on** | 73.8 → **23.9 ms** | 33.3% → **71.2%** | 1.50× → **3.47×** |
> | instances 48 | off → **on** | 126.2 → **52.8 ms** | 15.2% → **33.1%** | 1.18× → **1.49×** |
>
> **④ F(JIT/네이티브) 최종 기각.** 잔여 여유 = 상한 ÷ VM 실배속:
>
> | shape | VM 배속 | 상한 | **JIT 잔여** |
> |---|---|---|---|
> | in-module 48 | 2.13× | 3.47× | 1.63× |
> | **instances 48 (실 RTL 형태)** | 1.41× | 1.49× | **1.06×** |
>
> **실 RTL 형태에는 사실상 여유가 없다.** 융합 후에도 `total 52.8 / vm 17.5 / fallback 1.2` → **엔진이 64.6%** 다(넷 부기·propagate·NBA). 바디-측 백엔드는 그걸 못 건드린다. ⇒ BACKEND ④(cranelift JIT)는 **닫는다**.
>
> **다음 레버가 특정됐다 = 넷 개수 축소.** 융합은 복사 *assign* 을 없앴지만 중간 **net 자체**는 남는다(48단 × 단당 여러 넷). VCS 계열 flatten 의 나머지 절반이 정확히 이것이다. 단 이건 `--probe`/계층 VCD/`%m`/계층 참조가 지목하는 대상을 지우므로 **G2 목표와 정면 충돌** — 성능 트레이드오프가 아니라 목표 판정 사항이다.

## 7. 조건부 / 장기 (재진입 트리거 충족 시에만 승격 · 정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
> **🔴 정정 (2026-08-01, 같은 날) — 아래 "B 가 진짜 레버" 판정은 내 오독이었다. 실제 `try_compile` 로 재니 native-eval 은 이미 97% 를 컴파일한다.**
>
> 아래 인구조사는 `classify_binop` 의 7개 op 를 "지원 집합"으로 읽었는데, 그건 한 경로일 뿐이고 `native_eval::compile` 의 lowering 은 **비교·등가·case-등가·논리이항을 별도 arm 으로 이미 처리한다**(`compile.rs:336`, `:389`). 신규 `sim_engine::native_eval_coverage` 로 **실제 `try_compile` 을 호출해** 재측정:
>
> **picorv32+tb: codegen-able 바디의 assign RHS 691/709 = 97% 컴파일됨.**
>
> ⇒ **빠진 lane 은 병목이 아니다. B 는 다시 저ROI 다.**
>
> **그러면 왜 VM 이 1.04× 인가** — 정정된 답: 컴파일이 되는데도 안 빨라진다는 것은 **비용이 디스패치가 아니라 인터프리터와 VM 이 공유하는 프리미티브**에 있다는 뜻이다. 프로파일이 그걸 지목했다: `Value::resize`/`mask_top`/`from_packed` **30%** + net read **9%**. 그리고 doc-18 의 2026-06-07 항이 **이미 같은 것을 적어뒀다** — *"진짜 지배 비용은 bit-serial 처리 · 인터프리터·VM 공유 경로"*.
>
> ⇒ **F(JIT)도 같은 공유 경로를 상속하므로 상한을 못 가져간다 → F 는 닫힌 채로 유지되고, 이제 근거가 확실하다.**
> ⇒ **실물 RTL 의 레버는 백엔드가 아니라 공유 value/net 프리미티브다.**
>
> ⭐ **교훈(이번 세션 4번째 같은 형태)**: 계측이 자기 형태만큼만 본다. 이번엔 **코드를 읽어서 "지원 집합"을 추론**한 게 오독이었다 — 지원 여부는 **그 함수를 실제로 호출해서** 재야 한다.
>
> <details><summary>(오독이었던 원래 인구조사 — 기록 보존)</summary>
>
> **🔴 2026-08-01 — B 축(native-eval 잔여 lane)의 "저ROI" 판정은 벤치마크가 만든 착시였다. 실물 설계의 진짜 레버다.**
>
> `--backend vm` 으로 PicoRV32+TB 를 돌리고 샘플링하니 **인터프리터의 `eval_ctx` 가 self-time 56%** 였다. VM 이 native-eval 로 못 먹는 식을 전부 트리워크로 넘기고 있다는 뜻. 연산자 인구조사(`perf_real_design_operator_census`):
>
> | op | 개수 | native-eval |
> |---|---|---|
> | Add · Sub | 633 | ✅ |
> | **LogAnd** | **237** | ❌ BAIL |
> | **Eq** | **146** | ❌ BAIL |
> | **CaseEq** | **120** | ❌ BAIL |
> | **LogOr** | **70** | ❌ BAIL |
> | Ne·Shl·Shr·AShr·Lt·Gt·Ge | 31 | ❌ BAIL |
>
> **컴파일 가능 662/1266 = 52%**, 그리고 bail 은 **식 단위**라 트리 어딘가의 `Eq` 하나가 식 전체를 넘긴다 ⇒ 실효 커버리지는 52% 보다 훨씬 낮다. **미지원 604 중 573(95%)이 상위 4개(`&&`·`==`·`===`·`||`)** 다.
>
> ⭐ **왜 "저ROI" 로 판정됐었나** — 이 저장소의 벤치마크가 **native-eval 이 이미 지원하는 연산으로 쓰여 있었다**(`EXPR_HEAVY` = `+` 사슬 · `STRUCT_HEAVY` = select/concat/replicate). **벤치마크는 자기가 재도록 만들어진 것을 쟀다.** 실물 RTL 은 명령 디코드(`insn[6:0] == 7'b0110011`)와 조건 논리가 지배한다.
>
> ⇒ **F(JIT)의 "잔여 2.37×" 는 JIT 기회가 아니라 native-eval 의 빠진 lane 이다.** opcode 4개로 될 일에 JIT 을 사는 구조. **B 를 최우선으로 올리고 F 는 그 뒤에 재평가한다.**
>
> </details>

| BACKEND | ① ~~2-state 별도 모드~~ **기각 (2026-08-01 M1 실측: `unk` 평면 = 비용의 7%, 기준 30%)** — [preview/20](preview/20-cycle-mode-feasibility.md) · cycle-based 는 따라올 이유 없음(융합/levelize 실물 기각) ② PDES BSP 병렬(Amdahl 상한 T4≈2.5x) ③ native-eval 잔여 lane(signed>64·>128bit·sysfunc·real) ④ in-process JIT(cranelift-jit) 스파이크 | ① 대형 RTL 실수요 ② 지속 W≥64+grain≥200ns ③ 저ROI 상시 defer ④ **미평가** — P0a 후보에 없었다(§아래) |

> **🔴 바디-측 백엔드 축 최종 정산 (2026-07-31 C-GAIN 실측) — 축 자체를 닫는다.** G0 이 "C(allow-list 확장)의 **상한**은 2.84–4.24× 로 F 보다 크다"고 해서 **실현치**를 쟀다. C 가 흡수할 바디는 `#delay` 스티뮬러스 = eval-light 이므로, 활성당 작업량 스윕(`perf_work_per_body_crossover`, 엣지 100k 고정)으로 C 를 짓지 않고 답이 나온다: **1–2 문장/활성에서 vm/interp = 0.99×**(무승부 — 활성당 고정비인 레지스터 리스·프롤로그·디스패치가 상각 안 됨), 의미 있는 이득은 **8문장 이상**부터. 스티뮬러스 바디는 1–3 문장이다 ⇒ **C 실현치 = +0.2–0.3%**(sha256 총 37.7→37.6 ms · clock-bound 32.7→32.6 ms) vs 상한 2.84–4.24×. 게다가 resume-PC 상태기계(L)라는 새 silent-wrong 표면을 지불한다. **C = 기각.** ⇒ **A 출하 · B defer · C 기각 · F 기각**으로 바디-측 축은 소진. 남은 레버는 **엔진 축**(24–35% · 이미 두 라운드 수확)과 **스케줄 축**(D levelize · E flatten)뿐. 상세 = [preview/18](preview/18-acceleration-analysis.md) §C-GAIN.
>
> **BACKEND ④ 판정 (2026-07-31 G0 실측) — 스파이크 권고 철회.** `run_body` 스크래치 타이머로 **적격 바디의 wall-clock 비중 `f`** 를 재니(측정 후 되돌림), **클럭드 RTL 은 `f` = 31–55%** 였다(sha256-round 55.1% · clock-bound 31.1%). 어떤 바디-측 백엔드든 상한이 `1/(1−f)` = **1.45–2.23×** 인데 **VM 이 이미 1.15–1.49× 를 회수**했으므로 **F 의 잔여 여유는 1.26–1.50×**. cranelift 의존성 ~30개 + unsafe 표면 + 다세션 공수의 대가로는 안 맞는다 → **④ 는 defer**(재진입 트리거 = `f > 0.85` 인 실수요 워크로드, 예: 바디 안에 루프가 통째로 든 behavioral 연산 — 실측 벤치 3종이 그 형상이라 `f≈100%`). **C(allow-list 확장)가 F 보다 큰 레버**다(부적격 바디를 흡수하면 상한 2.84–4.24×) — 단 그 바디들은 eval-light 라 실현치는 별개로 재야 한다. 남는 **24–35% 는 엔진 작업**(settle·propagate·NBA·VCD)이라 C 도 F 도 못 건드린다. **결론: 78× 급은 바디 실행 속도가 아니라 스케줄 변경(D levelize·E flatten)에서만 나온다.** 상세 표 = [preview/18](preview/18-acceleration-analysis.md) §G0.
>
> **BACKEND ④ 배경 메모 (2026-07-31, 위 판정 이전).** P0a(2026-06-06)가 네이티브 방출을 기각한 근거는 전부 **소스 방출**에 특유하다(런타임 rustc/cc + libloading · host LLVM 재증명). **in-process JIT 은 후보에 오른 적이 없다** — 아키텍처 문서 6곳이 "후속 JIT 백엔드"를 예고만 하고 결정 기록엔 빠졌다. 실측: `cargo info cranelift-jit` → **0.121.2, `rust-version: 1.85.0`**(vita MSRV 정확 일치) · Apache-2.0 WITH LLVM-exception ⊂ vita `MIT OR Apache-2.0` · 순수 Rust(doc-03 §외부 의존성 정책 통과) · 의존성 build.rs 는 이미 허용(규칙은 **vita 자체 크레이트**에만). 결정성 핀 10개 중 F 가 실제로 건드리는 건 **#6 float 표면 동결**(회피법이 P3 계약에 이미 문서화 — JIT 이 그 함수를 직접 호출)과 **#9 unsafe**(`forbid(unsafe_code)` 없음 · prod 에 SAFETY 주석 unsafe 1건) **둘뿐**이다. `SchemaHash`/`format_version`/3-OS 바이트동일/`--locked` 는 무영향(백엔드는 `SimOpts` out-of-band). 최대 자산 = **P5 게이트가 이미 있다** — 세 번째 백엔드가 teeth 를 공짜로 상속. 남는 리스크: P9 allow-list 가 여전히 천장(JIT 도 같은 것을 상속하므로 적중률은 안 는다) · 이득 상한 A+F ≈ 6-15×(78× 는 levelize+flatten 없이 불가). **권고 = 전체 커밋이 아니라 lane 하나 스파이크(M · 2-3세션)로 P5 통과 여부를 사서 판정.**
| VHDL | VHDL 프론트엔드(9-value std_logic 매핑·별도 파서·GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원** — `$dumpfile("x.fst")`/`-o x.fst`; known-edge=소형 타임테이블 fst-writer [issue #4] loud 거부, preview/07 참조) |
| MVP-CUT | string concat-nonassign · wildcard assoc `[*]` · package internal-import/scoped-call 잔여 · cross-frame disable | 개별 수요 시 |

## 8. 비계획 (영구 비목표 · gap 아님)

- **DEFPARAM**(IEEE deprecated·`#(.param())`로 충분) · **IMPLICIT-NET**(정책=E3010 명시 에러) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).

## 9. 완료 이력 포인터

- 완료 슬라이스 상세 로그(§4.5.3~§4.5.134)·구 §0~§7 원문 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존).
- 탄 단위 내러티브·방법론 교훈 = [DEVLOG.md](DEVLOG.md)·ARCHIVE §3.
- 외부 호환성 리포트 1·2차 전말(A1~C1·EXT2 체인) = ARCHIVE §6·§6-2 — **잔여는 위 §3 "외부 리포트 잔여" 3건뿐**.
