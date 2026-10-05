Ewi_cac: same(wrong) post==ctrl.pre
Ewi_cacr: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | rc=1
   post: acr top.u8 r=00000000 b=2
   ivl : acr top.u8 r=00011111 b=2
Ewi_cae: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.u6] | rc=1
   post: ae top.u6 W=1
   ivl : ae top.u6 W=x
Ewi_caer: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.u9
   post: aer top.u9 W=1 b=2
   ivl : aer top.u9 W=x b=2
Ewi_cgen: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 generate-for exceeds the unroll cap (possible infinite loop) [in top.u3] | !VITA-E3010 generate-case scrutinee is not a constant [in top.u3] | rc=1
   post: gcs top.u3.c1 one | gfor top.u3.lp[0] k=0 | gfor top.u3.lp[1] k=1 | gif top.u3.one one
   ivl : gcs top.u3.c1 one | gfor top.u3.lp[0] k=0 | gfor top.u3.lp[1] k=1 | gif top.u3.one one
Ewi_ciar: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 instance array `lf` range is not a constant [in top.u5] | rc=1
   post: leaf top.u5.lf[0] | leaf top.u5.lf[1]
   ivl : leaf top.u5.lf[0] | leaf top.u5.lf[1]
Ewi_crep: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 parameter `RR` value is not a constant: the replication count 18446744073709551617 does no | rc=1
   post: rep top.u4 r=00000011 RR=00000011
   ivl : rep top.u4 r=00000011 RR=00000011
Ewi_ovrx1: silent->correct(=s2v,vl) post==ctrl.pre
   pre : ovr top.ua P=18446744073709551617 b=65 | ovr top.ue P=18446744073709551614 b=65
   post: ovr top.ua P=2 b=32 | ovr top.ue P=-1 b=32
   ivl : ovr top.ua P=2 b=33 | ovr top.ue P=-1 b=33
Ewi_ovrx2: silent->correct(=ivl,vl) post==ctrl.pre
   pre : ovrh top.ub P=0 b=4 | ovrh top.uc P=20000000000000000 b=66 | ovrh top.ud P=00 b=8
   post: ovrh top.ub P=1 b=4 | ovrh top.uc P=000000002 b=33 | ovrh top.ud P=01 b=8
   ivl : ovrh top.ub P=1 b=4 | ovrh top.uc P=000000002 b=33 | ovrh top.ud P=01 b=8
Ewi_pac: same(wrong) post==ctrl.pre
Ewi_pae: same(wrong) post==ctrl.pre
Ewi_pcl: silent->correct(=ivl,s2v,vl) post==ctrl.pre
   pre : gcl top.gm hit | pcl top miss
   post: gcl top.gm hit | pcl top hit
   ivl : gcl top.gm hit | pcl top hit
Ewi_piar: same(=ivl,s2v,vl) post==ctrl.pre
Ewi_prep: same(=ivl,s2v,vl) post==ctrl.pre
Ex_cac: same(wrong)
Ex_cacr: same(wrong)
Ex_cae: same(wrong)
Ex_caer: same(wrong)
Ex_cgen: same(=ivl,s2v,vl)
Ex_ciar: same(=ivl,s2v,vl)
Ex_crep: same(=ivl,s2v,vl)
Ex_ovrx1: same(=s2v,vl)
Ex_ovrx2: same(=ivl,vl)
Ex_pac: same(wrong)
Ex_pae: same(wrong)
Ex_pcl: same(=ivl,s2v,vl)
Ex_piar: same(=ivl,s2v,vl)
Ex_prep: same(=ivl,s2v,vl)
Gs_cac: same(wrong) post==ctrl.pre
Gs_cacr: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 undefined name `N` is not allowed in a constant range bound [in top.g[0].u8] | rc=1
   post: acr top.g[0].u8 r=00000000 b=1 | acr top.g[1].u8 r=00000000 b=2
   ivl : acr top.g[0].u8 r=00000111 b=1 | acr top.g[1].u8 r=00011111 b=2
Gs_cae: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 parameter `W` value is not a constant: undefined name `N` is not a constant [in top.g[0].u | !VITA-E3009 parameter `W` value is not a constant: undefined name `N` is not a constant [in top
   post: ae top.g[0].u6 W=0 | ae top.g[1].u6 W=1
   ivl : ae top.g[0].u6 W=x | ae top.g[1].u6 W=x
Gs_caer: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 parameter `W` value is not a constant: undefined name `N` is not a constant [in top.g[0].u | !VITA-E3009 parameter `W` value is not a constant: undefined name `N` is not a constant [in top
   post: aer top.g[0].u9 W=0 b=1 | aer top.g[1].u9 W=1 b=2
   ivl : aer top.g[0].u9 W=x b=1 | aer top.g[1].u9 W=x b=2
Gs_cgen: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3010 generate-case scrutinee is not a constant: undefined name `N` is not a constant [in top.g[ | !VITA-E3010 generate-for condition is not a constant: undefined name `N` is not a constant [in 
   post: gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
   ivl : gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
Gs_ciar: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 instance array `lf` range is not a constant [in top.g[0].u5] | !VITA-E3009 instance array `lf` range is not a constant [in top.g[1].u5] | rc=1
   post: leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
   ivl : leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
Gs_crep: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 parameter `RR` value is not a constant: the replication `{n{…}}` has no constant-fold arm  | rc=1
   post: rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
   ivl : rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
Gs_ovrx1: same(=ivl,s2v,vl) post==ctrl.pre
Gs_ovrx2: same(wrong) post==ctrl.pre
Gs_pac: same(wrong) post==ctrl.pre
Gs_pae: same(loud) post==ctrl.pre
Gs_pcl: silent->correct(=ivl,s2v,vl) post==ctrl.pre
   pre : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] miss
   post: gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
   ivl : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
Gs_piar: same(=ivl,s2v,vl) post==ctrl.pre
Gs_prep: same(=s2v,vl) post==ctrl.pre
Gw_cac: same(wrong) post==ctrl.pre
Gw_cacr: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | rc=1
   post: acr top.g[0].u8 r=00000000 b=1 | acr top.g[1].u8 r=00000000 b=2
   ivl : acr top.g[0].u8 r=00000111 b=1 | acr top.g[1].u8 r=00011111 b=2
Gw_cae: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[0].u6] | !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[1
   post: ae top.g[0].u6 W=0 | ae top.g[1].u6 W=1
   ivl : ae top.g[0].u6 W=x | ae top.g[1].u6 W=x
Gw_caer: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[
   post: aer top.g[0].u9 W=0 b=1 | aer top.g[1].u9 W=1 b=2
   ivl : aer top.g[0].u9 W=x b=1 | aer top.g[1].u9 W=x b=2
Gw_cgen: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 generate-for exceeds the unroll cap (possible infinite loop) [in top.g[0].u3] | !VITA-E3009 generate-for exceeds the unroll cap (possible infinite loop) [in top.g[1].u3] | !VITA-E3010 gene
   post: gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
   ivl : gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
Gw_ciar: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 instance array `lf` range is not a constant [in top.g[0].u5] | !VITA-E3009 instance array `lf` range is not a constant [in top.g[1].u5] | rc=1
   post: leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
   ivl : leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
Gw_crep: LOUD->VALUE(=ivl,s2v,vl) post==ctrl.pre
   pre : !VITA-E3009 parameter `RR` value is not a constant: the replication count 18446744073709551626 does no | rc=1
   post: rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
   ivl : rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
Gw_ovrx1: silent->correct(=ivl,s2v,vl) post==ctrl.pre
   pre : ovr top.g[0].ua P=18446744073709551626 b=65 | ovr top.g[0].ue P=18446744073709551623 b=65 | ovr top.g[1].ua P=18446744073709551626 b=65 | ovr top.g[1].ue P=18446744073709551623 b=65
   post: ovr top.g[0].ua P=1 b=32 | ovr top.g[0].ue P=-2 b=32 | ovr top.g[1].ua P=2 b=32 | ovr top.g[1].ue P=-1 b=32
   ivl : ovr top.g[0].ua P=1 b=32 | ovr top.g[0].ue P=-2 b=32 | ovr top.g[1].ua P=2 b=32 | ovr top.g[1].ue P=-1 b=32
Gw_ovrx2: wrong->wrong post==ctrl.pre
   pre : ovrh top.g[0].ub P=9 b=4 | ovrh top.g[0].uc P=20000000000000012 b=66 | ovrh top.g[0].ud P=09 b=8 | ovrh top.g[1].ub P=9 b=4 | ovrh top.g[1].uc P=20000000000000012 b=66 | ovrh top.g[1].ud P=09 b=8
   post: ovrh top.g[0].ub P=0 b=4 | ovrh top.g[0].uc P=000000000 b=33 | ovrh top.g[0].ud P=00 b=8 | ovrh top.g[1].ub P=1 b=4 | ovrh top.g[1].uc P=000000002 b=33 | ovrh top.g[1].ud P=01 b=8
   ivl : !2 error(s) during elaboration. | !error: Concatenation operand "i" has indefinite width. | rc=2
Gw_pac: same(wrong) post==ctrl.pre
Gw_pae: same(loud) post==ctrl.pre
Gw_pcl: silent->correct(=ivl,s2v,vl) post==ctrl.pre
   pre : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] miss
   post: gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
   ivl : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
Gw_piar: same(=ivl,s2v,vl) post==ctrl.pre
Gw_prep: same(=s2v,vl) post==ctrl.pre
Gwe_cac: same(wrong) post==ctrl.pre
Gwe_cacr: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | rc=1
   post: acr top.g[0].u8 r=00000000 b=1 | acr top.g[1].u8 r=00000000 b=2
   ivl : acr top.g[0].u8 r=00000111 b=1 | acr top.g[1].u8 r=00011111 b=2
Gwe_cae: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[0].u6] | !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[1
   post: ae top.g[0].u6 W=0 | ae top.g[1].u6 W=1
   ivl : ae top.g[0].u6 W=x | ae top.g[1].u6 W=x
Gwe_caer: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[
   post: aer top.g[0].u9 W=0 b=1 | aer top.g[1].u9 W=1 b=2
   ivl : aer top.g[0].u9 W=x b=1 | aer top.g[1].u9 W=x b=2
Gwe_cgen: LOUD->VALUE(=ivl,vl) post==ctrl.pre
   pre : !VITA-E3009 generate-for exceeds the unroll cap (possible infinite loop) [in top.g[0].u3] | !VITA-E3009 generate-for exceeds the unroll cap (possible infinite loop) [in top.g[1].u3] | !VITA-E3010 gene
   post: gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
   ivl : gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
Gwe_ciar: LOUD->VALUE(=ivl,vl) post==ctrl.pre
   pre : !VITA-E3009 instance array `lf` range is not a constant [in top.g[0].u5] | !VITA-E3009 instance array `lf` range is not a constant [in top.g[1].u5] | rc=1
   post: leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
   ivl : leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
Gwe_crep: LOUD->VALUE(=ivl,vl) post==ctrl.pre
   pre : !VITA-E3009 parameter `RR` value is not a constant: the replication count 18446744073709551626 does no | rc=1
   post: rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
   ivl : rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
Gwe_ovrx1: silent->correct(=ivl,vl) post==ctrl.pre
   pre : ovr top.g[0].ua P=18446744073709551626 b=65 | ovr top.g[0].ue P=18446744073709551623 b=65 | ovr top.g[1].ua P=18446744073709551626 b=65 | ovr top.g[1].ue P=18446744073709551623 b=65
   post: ovr top.g[0].ua P=1 b=32 | ovr top.g[0].ue P=-2 b=32 | ovr top.g[1].ua P=2 b=32 | ovr top.g[1].ue P=-1 b=32
   ivl : ovr top.g[0].ua P=1 b=32 | ovr top.g[0].ue P=-2 b=32 | ovr top.g[1].ua P=2 b=32 | ovr top.g[1].ue P=-1 b=32
Gwe_ovrx2: wrong->wrong post==ctrl.pre
   pre : ovrh top.g[0].ub P=9 b=4 | ovrh top.g[0].uc P=20000000000000012 b=66 | ovrh top.g[0].ud P=09 b=8 | ovrh top.g[1].ub P=9 b=4 | ovrh top.g[1].uc P=20000000000000012 b=66 | ovrh top.g[1].ud P=09 b=8
   post: ovrh top.g[0].ub P=0 b=4 | ovrh top.g[0].uc P=000000000 b=33 | ovrh top.g[0].ud P=00 b=8 | ovrh top.g[1].ub P=1 b=4 | ovrh top.g[1].uc P=000000002 b=33 | ovrh top.g[1].ud P=01 b=8
   ivl : !2 error(s) during elaboration. | !error: Concatenation operand "i" has indefinite width. | rc=2
Gwe_pac: same(wrong) post==ctrl.pre
Gwe_pae: same(loud) post==ctrl.pre
Gwe_pcl: silent->correct(=ivl,vl) post==ctrl.pre
   pre : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] miss
   post: gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
   ivl : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
Gwe_piar: same(=ivl,vl) post==ctrl.pre
Gwe_prep: same(=vl) post==ctrl.pre
Gwi_cac: same(wrong) post==ctrl.pre
Gwi_cacr: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | rc=1
   post: acr top.g[0].u8 r=00000000 b=1 | acr top.g[1].u8 r=00000000 b=2
   ivl : acr top.g[0].u8 r=00000111 b=1 | acr top.g[1].u8 r=00011111 b=2
Gwi_cae: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[0].u6] | !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[1
   post: ae top.g[0].u6 W=0 | ae top.g[1].u6 W=1
   ivl : ae top.g[0].u6 W=x | ae top.g[1].u6 W=x
Gwi_caer: LOUD->VALUE(NO ORACLE MATCH) post==ctrl.pre
   pre : !VITA-E3009 `N` is wider than 64 bits, so it has no integral constant value for a width / range bound  | !VITA-E3009 parameter `W` value is not a constant: `fae(…)` has no constant-fold arm [in top.g[
   post: aer top.g[0].u9 W=0 b=1 | aer top.g[1].u9 W=1 b=2
   ivl : aer top.g[0].u9 W=x b=1 | aer top.g[1].u9 W=x b=2
Gwi_cgen: LOUD->VALUE(=ivl,vl) post==ctrl.pre
   pre : !VITA-E3009 generate-for exceeds the unroll cap (possible infinite loop) [in top.g[0].u3] | !VITA-E3009 generate-for exceeds the unroll cap (possible infinite loop) [in top.g[1].u3] | !VITA-E3010 gene
   post: gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
   ivl : gcs top.g[0].u3.c0 zero | gcs top.g[1].u3.c1 one | gfor top.g[0].u3.lp[0] k=0 | gfor top.g[1].u3.lp[0] k=0 | gfor top.g[1].u3.lp[1] k=1 | gif top.g[0].u3.oth other | gif top.g[1].u3.one one
Gwi_ciar: LOUD->VALUE(=ivl,vl) post==ctrl.pre
   pre : !VITA-E3009 instance array `lf` range is not a constant [in top.g[0].u5] | !VITA-E3009 instance array `lf` range is not a constant [in top.g[1].u5] | rc=1
   post: leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
   ivl : leaf top.g[0].u5.lf[0] | leaf top.g[1].u5.lf[0] | leaf top.g[1].u5.lf[1]
Gwi_crep: LOUD->VALUE(=ivl,vl) post==ctrl.pre
   pre : !VITA-E3009 parameter `RR` value is not a constant: the replication count 18446744073709551626 does no | rc=1
   post: rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
   ivl : rep top.g[0].u4 r=00000001 RR=00000001 | rep top.g[1].u4 r=00000011 RR=00000011
Gwi_ovrx1: silent->correct(=ivl,vl) post==ctrl.pre
   pre : ovr top.g[0].ua P=18446744073709551626 b=65 | ovr top.g[0].ue P=18446744073709551623 b=65 | ovr top.g[1].ua P=18446744073709551626 b=65 | ovr top.g[1].ue P=18446744073709551623 b=65
   post: ovr top.g[0].ua P=1 b=32 | ovr top.g[0].ue P=-2 b=32 | ovr top.g[1].ua P=2 b=32 | ovr top.g[1].ue P=-1 b=32
   ivl : ovr top.g[0].ua P=1 b=32 | ovr top.g[0].ue P=-2 b=32 | ovr top.g[1].ua P=2 b=32 | ovr top.g[1].ue P=-1 b=32
Gwi_ovrx2: wrong->wrong post==ctrl.pre
   pre : ovrh top.g[0].ub P=9 b=4 | ovrh top.g[0].uc P=20000000000000012 b=66 | ovrh top.g[0].ud P=09 b=8 | ovrh top.g[1].ub P=9 b=4 | ovrh top.g[1].uc P=20000000000000012 b=66 | ovrh top.g[1].ud P=09 b=8
   post: ovrh top.g[0].ub P=0 b=4 | ovrh top.g[0].uc P=000000000 b=33 | ovrh top.g[0].ud P=00 b=8 | ovrh top.g[1].ub P=1 b=4 | ovrh top.g[1].uc P=000000002 b=33 | ovrh top.g[1].ud P=01 b=8
   ivl : !2 error(s) during elaboration. | !error: Concatenation operand "i" has indefinite width. | rc=2
Gwi_pac: same(wrong) post==ctrl.pre
Gwi_pae: same(loud) post==ctrl.pre
Gwi_pcl: silent->correct(=ivl,vl) post==ctrl.pre
   pre : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] miss
   post: gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
   ivl : gcl top.g[0].gd miss | gcl top.g[1].gm hit | pcl top.g[0] miss | pcl top.g[1] hit
Gwi_piar: same(=ivl,vl) post==ctrl.pre
Gwi_prep: same(=vl) post==ctrl.pre
Gx_cac: same(wrong)
Gx_cacr: same(wrong)
Gx_cae: same(wrong)
Gx_caer: same(wrong)
Gx_cgen: same(=ivl,s2v,vl)
Gx_ciar: same(=ivl,s2v,vl)
Gx_crep: same(=ivl,s2v,vl)
Gx_ovrx1: same(=ivl,s2v,vl)
Gx_ovrx2: same(wrong)
Gx_pac: same(wrong)
Gx_pae: same(loud)
Gx_pcl: same(=ivl,s2v,vl)
Gx_piar: same(=ivl,s2v,vl)
Gx_prep: same(=s2v,vl)
{'same': 55, 'LOUD->VALUE': 30, 'silent->correct': 10, 'wrong->wrong': 3}
