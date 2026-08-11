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
| **★** | §5 | **③층 — 쓰기 퍼널의 한 워드 진입점 ✅(§4.5.332) picorv32 1.53×. 다음 = 대입당 디스패치, 그다음 S3** | 1 | 실측 | ⭐ 정본 `write_lvalue` 가 자기 정규화를 마친 뒤 **목적지가 아레나 워드 하나면** 평면 진입점으로 위임. picorv32 0.633→**0.594(6.2%)**, 퍼널+`resize` **20.6→12.5%**(`write_chunk` 는 상위 22 에서 사라졌다). ⭐⭐ **표적은 census 가 정했고 그것이 내 계획의 절반을 버렸다** — 비트 직렬은 **1.4%**, 통짜 저장이 **98.6%** 였다. 추출 셋 전부 정본이 위임(`resize_word`·`chunk_lsb`·`chunk_elem`). ⭐⭐ **내 스윕을 리뷰해 구멍 둘**(엣지 감도 0개 → 글리치/엣지 미도달 · `is_real`/`is_str` 미설정 = admission 논거 그 자체)을 찾아 막았고, 그 뒤 뮤테이션 **12/12 사망**. 정본 = **[preview/21 §0.-6](preview/21-tier3-native-backend.md)** |
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
- **🔴 사이즈 캐스트 주변의 pre-existing 셋(PRE==POST·같은 슬라이스가 실측)** — ⓐ **저비트 폐쇄는 2-state 논증이다**: `logic [7:0] a=8'bxxxx_0011` 에서 `2'(a+1)` 이 vita `00` / iverilog `xx`(캐스트 없는 `a+1` 은 vita 도 `xx` 로 정확 = 좁히기만의 문제). 비트연산과 `<<` 는 4-state 에서도 폐쇄(4,116칸 발산 0)라 절반만 틀린 논증이다. ~~ⓑ **`**` 는 음수 지수에서 폐쇄가 아니다**~~ **RESOLVED**(§4.5.319) — `Pow` 가 `Div|Mod|Shr|AShr` 폭-프로브 가지로 옮겨졌다(회귀 0 · FIXED 72). ⓒ **fill 리터럴이 `lower_size_ctx` 를 거치면 캐스트 문맥을 잃는다**(`8'(a+'1)` vita `…110` / 오라클 `…100` · `8'('1>>1)` vita `0` / 오라클 `01111111` · **비트연산 철자도 같다**: `4'('1 ^ a)` a=4'hD 에서 vita `1100` / iverilog `0010`, `9'(('1+one) % (sb**3))` vita `000000010` / iverilog `000000000` — 캐스트 없이는 둘 다 일치) — Size arm 주석이 *"fill 은 N 으로 자란다"* 라고 적은 바로 그 경로를 `is_size_ctx_operation` 게이트가 우회시킨다.
- **🔴 크기 캐스트가 파라미터·패키지 파라미터·함수 호출 leaf 를 못 본다 — 그리고 고치려면 AST self-폭 패스가 먼저다**(pre-existing·PRE==POST·오라클 ✓·§4.5.318 이 지어서 재고 **되돌렸다**). `ast_ctx_signed` 가 leaf 하나라도 `None` 이면 캐스트가 문맥 하강을 **통째로** 건너뛰므로, 그 leaf 종류에서는 §4.5.212 의 carry 결함이 그대로 살아 있다 — 63칸 매트릭스에서 **35칸 발산**(`8'(P*Q)` = 13 / 오라클 45 · `16'(P<<4)` = 0 / 오라클 3840 · 넷과 `localparam int` 는 0/9). ⚠️ **해석기를 붙이는 것만으로는 안 된다**(전부 실측):
  - ⓐ 붙이면 그 트리가 `lower_size_ctx` 로 들어가는데, **안쪽 부분식의 self 폭이 `n` 을 넘으면 폭 모델이 틀린다**(§4.5.316 의 프로브도 `n` 에서 다시 잰다) → 6,720칸 중 **42칸 correct→silent-wrong**(`4'(13 + (PS>>2))` PRE −4 = 두 오라클 / POST 0 · net 쌍둥이는 PRE 도 틀림 = 맞바꾸기). **선행조건 = 낮추지 않고 트리 전역 self 폭을 답하는 AST 패스**(§2 의 ⓐ·ⓑ·real+fill 84칸과 같은 뿌리).
  - ⓑ **함수 호출 leaf 는 `extend_to` 가 두 번 참조한다**(`Select{base:e}` + `Concat[fill,e]`) → `40'(rnd(0)+0)` 이 함수를 **두 번** 부르고 두 `$random` draw 를 한 값에 **섞는다**(exit 0). §4.5.310 이 인덱스에서 겪은 그 부류 — **복제 가능성 술어로 게이트**하거나 값을 한 번 묶어야 한다.
  - ⓒ **함수 formal·함수/블록 local 은 결합 바인딩 집합에 없다** → lowering 을 흉내 내 결합 키를 먼저 잡으면 그것들을 지나쳐 **동명 모듈 파라미터의 부호**를 가져온다(52칸 중 11 회귀). 반대로 넷 우선(현재)이면 generate 스코프 localparam 이 바깥 넷에 가린다(`16'(X*S)` = 3 / iverilog 195, PRE==POST). **어느 고정 순서도 규칙이 아니다** — lowering 의 실제 결정 절차를 그대로 따라야 하고, 거기엔 `subst_lookup`/`out_subst_lookup` 같은 인라인 치환 경로가 더 있다.
- **🔴 크기 캐스트의 real 이 fill 과 만나면 아직 조용하다**(pre-existing·PRE==POST·오라클 ✓·§4.5.317 적대 2렌즈 실측). §4.5.317 이 **퍼널이 만드는 모든 피연산자**를 막았지만, 판별자 **셋이 동시에 성립**하면 퍼널에 애초에 안 들어간다 — 반대편이 **fill(`'0`/`'1`)** ∧ real 원천이 **평범한 real 넷이 아님**(`parameter real`·real 리터럴·real 함수 반환·`$signed(r)`·`$realtime`·`$sqrt(r)`) ∧ 연산자가 **real 비전파**(`& | ^ << >> >>> %`). 288칸 매트릭스 중 **84칸**(`4'(RP ^ '0)`·`4'($sqrt(r) & '1)`·`4'(b >>> (rt ^ '0))` 이 전부 exit 0·iverilog 는 전부 거부). 두 자물쇠가 같이 열려야 샌다: `ast_ctx_signed` 가 그 leaf 에서 `None` 이라 `lower_size_ctx` 를 **안 타고**, `expr_is_real` 의 `Binary` 팔에는 **비트/시프트/`%` 가 없다**. 하나만 고치면 다른 하나가 남으므로 **인스턴스가 아니라 CLASS** 이고, 선행조건은 위 ⓐ/ⓑ 와 같은 **트리 전역 AST self-폭/도메인 패스**다.
- **`$signed(real)`/`$unsigned(real)` 이 위치 의존적이다**(pre-existing·PRE==POST·§4.5.317 발굴). 캐스트 안 15자리는 거부하지만 7자리는 exit 0 — 평범한 산술(`$signed(r)*2` → 15)·`%0d`/`%0f` 포맷·int/real 대입. iverilog 는 `The argument to $signed must be a vector type` 로 **전부** 거부한다. 곁: **2인자 `$signed(r, u)` 를 조용히 받는다**(iverilog: *"takes exactly one(1) argument"*) — §4.5.317 은 그 형태에서 **어느 슬롯이든** real 이면 캐스트를 거부하도록 맞췄지만, 스펠링 자체의 거부는 별건이다.
- **🔴 elaborate 상수접기가 `**` 지수를 자기결정하지 않는다**(pre-existing·PRE==POST·오라클 ✓·§4.5.319 적대 리뷰 발굴). `localparam signed [15:0] P = 4'sd3 ** -4'sd8` 이 vita **6561** / iverilog **0** — 지수 식의 self-determined 값이 음수인데 넓은 param 문맥에서 양수로 읽힌다(128칸 중 50칸). **같은 소스가 param 과 런타임에서 다른 값**을 낸다(런타임은 §4.5.319 이후 0 으로 정답). 비음수 지수는 정상(`8'd3 ** 4'd11` = 46075 = iverilog). 자리 = `const_eval.rs` 의 폭/부호 모델(`const_fn.rs::const_pow_i64` 자체는 Table 11-6 을 엔진과 일치하게 구현하고 있다 — 틀린 건 그 위에서 지수를 넓히는 쪽). §4.5.319 가 다른 다섯 철자(엔진 `arith`·`lower_expr_ctx`·`expr_size_ctx`·연속대입 하강·bytecode `const_pow_exponent`)를 전부 닫았으므로 **이것이 마지막 하나**다.
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

### 5.1 ★★ 다음 축 — **실행기 둘로**(interp = 오라클 · native = 제품) · 오너 지시 2026-08-10

**목표 형태**: `--backend interp`(테스트 전용 오라클) + `--backend native`(제품). **②층 VM 은퇴.**

**왜**: ⓑ(코드젠)가 언젠가 필요해지면 비싼 것은 cranelift 가 아니라 **4-state 규칙이 두 철자가 되는
것**이다. 실행기가 둘이면 그 긴장이 *"두 번째 철자를 만드는 일"* 에서 *"오라클이 지키는 코드
생성기를 만드는 일"* 로 바뀐다. **상용(VCS·Xcelium)이 실행기 하나로 되는 이유는 오라클을 바깥에서
얻기 때문**(수십 년 회귀·고객 설계·경쟁 도구 크로스체크)이고, vita 는 그 자산이 없으니 인터프리터를
**영구 오라클**로 남긴다.

| 단계 | 무엇 | 게이트 / 중단 판정 |
|---|---|---|
| **V0** ✅ | **커버리지 격차 측정 (2026-08-10 · §4.5.336).** 기본 백엔드를 `Native` 로 flip 하고 전 스위트를 돌렸다. **결과 = 아래 두 표.** 스캐폴드는 되돌렸다(트리 변경 0) | — (계기). **이 숫자가 V1 의 슬라이스 목록이다** |
| **V1** 진행중 | **커버리지 확장.** 아래 §5.1-b 의 **측정된 순서**대로 닫는다. **슬라이스 1(SVA) 완료 — 54.7% → 66.8%**(§4.5.337) | 슬라이스마다 **VM 과 바이트 동일** |
| **V2** | **VM 은퇴.** native 가 VM 범위를 덮으면 `--backend vm` 을 제품 표면에서 제거하고 `native_eval`(두 번째 식 평가기)·`CompileCtx.natives`·tier-2 `jit.rs` 를 지운다 | **전 스위트가 `--backend native` 로 초록**이 선행조건. 못 덮으면 V1 로 되돌아간다 |
| **V3** | **interp 강등.** `--backend interp` 를 제품 표면에서 빼고 **테스트 전용 오라클**로. `compute_effect`/`apply_effect` 의 `Kernel` 제네릭은 **유지** — 그것이 오라클의 근거다 | 차분 게이트가 interp↔native 로 남는지 확인 · 성능 최적화 대상에서 제외 |

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
| `dyn_elem_string` | **206** |
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
