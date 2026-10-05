| cell | iv | sv2v | vl | PRE | P3 | PRE→P3 |
|---|---|---|---|---|---|---|
| B1_blklocal_fwd | bits=4 t=15 K=4 | bits=8 t=255 K=8 | bits=8 t=255 K=8 | bits=4 t=15 K=8 | bits=8 t=255 K=8 | silent→correct =sv+vl |
| B1n_blklocal_fwd | ERR:B1n_blklocal_fwd.sv:4: error: Unable to bind parameter ` | bits=8 t=255 K=8 | bits=8 t=255 K=8 | ERR:VITA-E3009:undefined name `K` is not allowed in a consta | bits=8 t=255 K=8 | loud→value =sv+vl |
| C01_ctl_plain | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 then bits= | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 then bits= | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 then bits= | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 then bits= | L0 c0 V=10 / L0 else bits=4 w=0 / L1 cd V=20 / L1 then bits= | same |
| C02_ctl_shadowbwd | eight bits=8 K=8 | eight bits=8 K=8 | eight bits=8 K=8 | eight bits=8 K=8 | eight bits=8 K=8 | same |
| E01_enumlbl_prebind | def 9 bits=4 P=18446744073709551617 | one 200 bits=8 P=1 | one 200 bits=8 P=1 | one 200 bits=8 P=1 | one 200 bits=8 P=1 | same |
| E02_bitsnet_prebind | eight 200 bits=8 P=8 | eight 200 bits=8 P=8 | eight 200 bits=8 P=8 | def 9 bits=4 P=4 | def 9 bits=4 P=4 | same (no oracle) |
| E03_cfn_prebind | six 200 bits=8 P=6 | six 200 bits=8 P=6 | six 200 bits=8 P=6 | six 200 bits=8 P=6 | six 200 bits=8 P=6 | same |
| E03b_cfn_err | P=6 | P=6 | P=6 | ERR:VITA-E3009:generate-scope parameter `P` value is not a c | ERR:VITA-E3009:generate-scope parameter `P` value is not a c | same (no oracle) |
| E04_badrange | ERR:E04_badrange.sv:3: error: Unable to bind parameter `Nope | ERR:E04_badrange.sv2v.v:4: error: Unable to bind parameter ` | ERR:%Error: E04_badrange.sv:3:23: Can't find definition of v | ERR:VITA-E3009:undefined name `Nope` is not allowed in a con | ERR:VITA-E3009:undefined name `Nope` is not allowed in a con | same (no oracle) |
| E06_genvar_prebind | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 two 200 bits=8 / L2 def 9 bits=4 | same |
| E08_fwdbits_param | ERR:E08_fwdbits_param.sv:3: error: Unable to bind parameter  | eight 200 bits=8 | eight 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a constant: un | ERR:VITA-E3010:generate-case scrutinee is not a constant: un | same (no oracle) |
| E09_import_sh | def 9 bits=4 | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| E12_realsh | P=3 | P=7 | P=7 | ERR:VITA-E3009:generate-scope parameter `P` value is not a c | P=7 | loud→value =sv+vl |
| E12b_realsh_case | ERR:E12b_realsh_case.sv:5: error: Cannot evaluate genvar cas | seven 200 bits=8 | seven 200 bits=8 | ERR:VITA-E3009:generate-scope parameter `P` value is not a c | seven 200 bits=8 | loud→value =sv+vl |
| F1_forfwd_sh | L0 w=10 | L0 w=10 / L1 w=11 / L2 w=12 | L0 w=10 / L1 w=11 / L2 w=12 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].L[1].w` [i | L0 w=10 / L1 w=11 / L2 w=12 | loud→value =sv+vl |
| F2_forfwd | ERR:F2_forfwd.sv:3: error: Unable to bind parameter `N' in ` | L0 w=10 / L1 w=11 / L2 w=12 | L0 w=10 / L1 w=11 / L2 w=12 | ERR:VITA-E3010:generate-for condition is not a constant: und | L0 w=10 / L1 w=11 / L2 w=12 | loud→value =sv+vl |
| F3_forfwd_sh_nonet | L0 | L0 / L1 / L2 | L0 / L1 / L2 | L0 / L1 / L2 | L0 / L1 / L2 | same |
| G1_enum_fwdparam | ERR:G1_enum_fwdparam.sv:3: error: Unable to bind parameter ` | bits=6 v=1 | bits=6 v=1 | ERR:VITA-E3009:undefined name `K` is not allowed in a consta | ERR:VITA-E3010:undeclared net/variable `top.gb[0].EB` [in to | loud→loud =none |
| H1_genfn_fwd | f=4 | f=8 | f=8 | f=8 | f=8 | same |
| I1_iffwd | ERR:I1_iffwd.sv:3: error: Unable to bind parameter `K' in `t | then 200 bits=8 | then 200 bits=8 | ERR:VITA-E3010:generate-if condition is not a constant: unde | then 200 bits=8 | loud→value =sv+vl |
| I2_iffwd_sh | else 9 bits=4 | then 200 bits=8 | then 200 bits=8 | then 8 bits=4 | then 200 bits=8 | silent→correct =sv+vl |
| I3_iffwd_sh_rev | then 200 bits=8 | else 9 bits=4 | else 9 bits=4 | else 9 bits=8 | else 9 bits=4 | silent→correct =sv+vl |
| I4_iffwd_sh_nonet | else | then | then | then | then | same |
| M1_modchain | ERR:M1_modchain.sv:2: error: Unable to bind parameter `B' in | A=3 B=3 | A=3 B=3 | ERR:VITA-E3009:parameter `A` value is not a constant: undefi | ERR:VITA-E3009:parameter `A` value is not a constant: undefi | same (no oracle) |
| M2_genchain | ERR:M2_genchain.sv:3: error: Unable to bind parameter `B' in | A=3 B=3 | A=3 B=3 | ERR:VITA-E3009:generate-scope parameter `A` value is not a c | A=3 B=3 | loud→value =sv+vl |
| M3_genchain_sh | ERR:M3_genchain_sh.sv:10: error: Unable to bind parameter `R | five 5 bits=6 P=5 | five 5 bits=6 P=5 | ERR:VITA-E3009:generate-scope parameter `Q` value is not a c | five 5 bits=6 P=5 | loud→value =sv+vl |
| M4_chain2sh | seven P=7 | five P=5 | five P=5 | five P=5 | five P=5 | same |
| M5_cycle | ERR:M5_cycle.sv:3: error: Recursive parameter reference foun | ERR:M5_cycle.sv2v.v:4: error: Recursive parameter reference  | ERR:%Error: M5_cycle.sv:4:16: Variable's initial value is ci | ERR:VITA-E3009:generate-scope parameter `A` value is not a c | ERR:VITA-E3009:generate-scope parameter `A` value is not a c | same (no oracle) |
| N1_netfwd | ERR:N1_netfwd.sv:3: error: Unable to bind parameter `K' in ` | K=8 bits=8 w=255 | K=8 bits=8 w=255 | ERR:VITA-E3009:undefined name `K` is not allowed in a consta | K=8 bits=8 w=255 | loud→value =sv+vl |
| N2_netfwd_sh | K=4 bits=4 w=15 | K=8 bits=8 w=255 | K=8 bits=8 w=255 | K=8 bits=4 w=15 | K=8 bits=8 w=255 | silent→correct =sv+vl |
| N2m2_modscope_case | four | def | def | def | def | same |
| N2m_modscope_sh | bits=4 w=15 | bits=8 w=255 | bits=8 w=255 | bits=8 w=255 | bits=8 w=255 | same |
| N2p_netonly_sh | bits=4 w=15 | bits=8 w=255 | bits=8 w=255 | bits=4 w=15 | bits=8 w=255 | split move =sv+vl |
| P1c_procread_fwd | K=4 v=4 / top.gb.u P=4 | K=8 v=8 / top.gb.u P=8 | K=8 v=8 / top.gb.u P=8 | K=8 v=8 / top.gb.u P=8 | K=8 v=8 / top.gb.u P=8 | same |
| Q65_catalog | P=2 bits=3 | P=11 bits=12 | P=11 bits=12 | P=11 bits=3 | P=11 bits=12 | silent→correct =sv+vl |
| Q65b_nested_begin | N=1 | N=5 | N=5 | N=1 | N=1 | same |
| S1_strsh | def 9 bits=4 | s 200 bits=8 | s 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a constant: a  | ERR:VITA-E3010:generate-case scrutinee is not a constant: a  | same (no oracle) |
| V01_d1p | ERR:V01_d1p.sv:5: error: Unable to bind parameter `K' in `to | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V01m_modfwd | ERR:V01m_modfwd.sv:4: error: Unable to bind parameter `K' in | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | same |
| V02_back | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | same |
| V03_scrfwd | ERR:V03_scrfwd.sv:3: error: Unable to bind parameter `S' in  | k 200 bits=8 | k 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a constant: un | k 200 bits=8 | loud→value =sv+vl |
| V04_scrfwd_sh | one 1 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | k 200 bits=8 | same |
| V04x_scrfwd_sh | one 1 bits=6 | k 200 bits=8 | k 200 bits=8 | k 8 bits=6 | k 200 bits=8 | silent→correct =sv+vl |
| V05a_r1 | def 9 bits=4 | k 200 bits=8 | k 200 bits=8 | def 9 bits=4 | def 9 bits=4 | same |
| V05b_r1b | k 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same |
| V05c_r1p | k 200 bits=8 | def 9 bits=4 | def 9 bits=4 | def 9 bits=8 | def 9 bits=4 | silent→correct =sv+vl |
| V05d_r1q | def 9 bits=4 | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V06_C1 | def 9 bits=4 P=10000000000000005 | p 200 bits=8 P=00000000000000003 | p 200 bits=8 P=00000000000000003 | p 8 bits=4 P=10000000000000005 | p 200 bits=8 P=00000000000000003 | silent→correct =sv+vl |
| V06n_C1narrow | def 9 bits=4 P=5 | p 200 bits=8 P=3 | p 200 bits=8 P=3 | p 8 bits=4 P=3 | p 200 bits=8 P=3 | silent→correct =sv+vl |
| V07_U1 | top.uA.a a / top.uB.a a | top.uA.a a / top.uB.a a | top.uA.a a / top.uB.a a | top.uA.d def / top.uB.d def | top.uA.d def / top.uB.d def | same (no oracle) |
| V08_nested | ERR:V08_nested.sv:5: error: Unable to bind parameter `K' in  | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V08b_nestedcase | ERR:V08b_nestedcase.sv:6: error: Unable to bind parameter `K | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V08c_nestedouterfwd | ERR:V08c_nestedouterfwd.sv:3: error: Unable to bind paramete | k 200 bits=8 | k 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a constant: un | k 200 bits=8 | loud→value =sv+vl |
| V09_forcase | ERR:V09_forcase.sv:4: error: Unable to bind parameter `K' in | L0 def 9 bits=4 / L1 k 201 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 k 201 bits=8 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 k 9 bits=4 / L2 def 9 bits=4 | L0 def 9 bits=4 / L1 k 201 bits=8 / L2 def 9 bits=4 | silent→correct =sv+vl |
| V09b_forcase_iter | ERR:V09b_forcase_iter.sv:4: error: Unable to bind parameter  | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 202 bits=8 | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 202 bits=8 | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 10 bits=4 | L0 def 9 bits=4 / L1 def 9 bits=4 / L2 k 202 bits=8 | silent→correct =sv+vl |
| V10_iarr | ERR:V10_iarr.sv:4: error: Unable to bind parameter `K' in `t | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bits=8 | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bits=8 | top.u[0].gb.g k 8 bits=4 / top.u[1].gb.g k 8 bits=4 | top.u[0].gb.g k 200 bits=8 / top.u[1].gb.g k 200 bits=8 | silent→correct =sv+vl |
| V12a_ovr | ERR:V12a_ovr.sv:4: error: Unable to bind parameter `K' in `t | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 8 bits=4 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | silent→correct =sv+vl |
| V12b_defparam | ERR:V12b_defparam.sv:4: error: Unable to bind parameter `K'  | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 8 bits=4 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | silent→correct =sv+vl |
| V12c_ovr_ctl | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | top.uA.gb.g k 200 bits=8 / top.uB.gb.g def 9 bits=4 | same |
| V13_multilab | ERR:V13_multilab.sv:4: error: Unable to bind parameter `K' i | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V14_fwd_nomatch | ERR:V14_fwd_nomatch.sv:4: error: Unable to bind parameter `K | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same |
| V14b_fwd_nodef | ERR:V14b_fwd_nodef.sv:5: error: Unable to bind parameter `K' | k 9 bits=4 | k 9 bits=4 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].g2[0].w` [ | k 9 bits=4 | loud→value =sv+vl |
| V15_laterblk | ERR:V15_laterblk.sv:4: error: A hierarchical reference (`gb. | ERR:V15_laterblk.sv2v.v:5: error: A hierarchical reference ( | ERR:%Error: Internal Error: V15_laterblk.sv:4:10: ../V3Const | def 9 bits=4 | def 9 bits=4 | same (no oracle) |
| V15b_sibling | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same |
| V16_hier | ERR:V16_hier.sv:4: error: Unable to bind parameter `K' in `t | h 200 bits=4 | h 200 bits=8 | h 8 bits=4 | h 200 bits=8 | silent→correct =vl |
| V17_difflbl | ERR:V17_difflbl.sv:4: error: Unable to bind parameter `K' in | k 200 bits=8 | k 200 bits=8 | ERR:VITA-E3010:undeclared net/variable `top.gb[0].gk[0].w` [ | k 200 bits=8 | loud→value =sv+vl |
| V18_inst | ERR:V18_inst.sv:6: error: Unable to bind parameter `K' in `t | k 200 bits=8 / top.gb.g.u child-a | k 200 bits=8 / top.gb.g.u child-a | k 8 bits=4 / top.gb.g.u child-a | k 200 bits=8 / top.gb.g.u child-a | silent→correct =sv+vl |
| V19_func | ERR:V19_func.sv:4: error: Unable to bind parameter `K' in `t | k f=101 | k f=101 | k f=8 | k f=101 | silent→correct =sv+vl |
| V20_armlp | ERR:V20_armlp.sv:4: error: Unable to bind parameter `K' in ` | k 200 bits=8 W=8 | k 200 bits=8 W=8 | k 8 bits=4 W=8 | k 200 bits=8 W=8 | silent→correct =sv+vl |
| V21_armfor | ERR:V21_armfor.sv:4: error: Unable to bind parameter `K' in  | k j0 200 bits=8 / k j1 201 bits=8 | k j0 200 bits=8 / k j1 201 bits=8 | k j0 8 bits=4 / k j1 9 bits=4 | k j0 200 bits=8 / k j1 201 bits=8 | silent→correct =sv+vl |
| V22_armvar | ERR:V22_armvar.sv:4: error: Unable to bind parameter `K' in  | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V23_armenum | ERR:V23_armenum.sv:4: error: Unable to bind parameter `K' in | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V24_region | ERR:V24_region.sv:3: error: Unable to bind parameter `K' in  | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V24b_region_net | ERR:V24b_region_net.sv:2: error: Unable to bind parameter `K | bits=8 K=8 | bits=8 K=8 | ERR:VITA-E3009:undefined name `K` is not allowed in a consta | ERR:VITA-E3009:undefined name `K` is not allowed in a consta | same (no oracle) |
| V24c_region_nested | ERR:V24c_region_nested.sv:4: error: Unable to bind parameter | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V24s_region_import | ERR:V24s_region_import.sv:9: error: 'K' has already been imp | def | def | def | def | same |
| V25_iface | ERR:V25_iface.sv:4: error: Unable to bind parameter `K' in ` | i 200 bits=4 | i 200 bits=8 | ERR:VITA-E3009:generate blocks inside an interface are outsi | ERR:VITA-E3009:generate blocks inside an interface are outsi | same (no oracle) |
| V26_xlabel | ERR:V26_xlabel.sv:5: error: Unable to bind parameter `K' in  | k 200 bits=8 | ERR:%Error: V26_xlabel.sv:4:7: Use of x/? constant in genera | k 8 bits=4 | k 200 bits=8 | silent→correct =sv |
| V27_enumfwd | ERR:V27_enumfwd.sv:4: error: Unable to bind parameter `EB' i | eb 200 bits=8 | eb 200 bits=8 | def 9 bits=4 | def 9 bits=4 | same (no oracle) |
| V28_bitsfwd | def 9 bits=4 | k8 200 bits=8 | k8 200 bits=8 | ERR:VITA-E3010:generate-case scrutinee is not a constant: un | ERR:VITA-E3010:generate-case scrutinee is not a constant: un | same (no oracle) |
| V28b_bitsfwd_sh | def 9 bits=4 | k8 200 bits=8 | k8 200 bits=8 | def 9 bits=4 | def 9 bits=4 | same |
| V28c_ifbits_sh | else 9 bits=4 | then 200 bits=8 | then 200 bits=8 | else 9 bits=4 | else 9 bits=4 | same |
| V29_genfn | f 200 bits=8 | f 200 bits=8 | ERR:%Error: V29_genfn.sv:8:32: Constant function may not be  | f 200 bits=8 | f 200 bits=8 | same |
| V30_strfwd | ERR:V30_strfwd.sv:4: error: Unable to bind parameter `S' in  | s 200 bits=8 | s 200 bits=8 | s 8 bits=4 | s 200 bits=8 | silent→correct =sv+vl |
| V30b_realfwd | ERR:V30b_realfwd.sv:4: error: Unable to bind parameter `R' i | ERR:V30b_realfwd.sv2v.v:5: error: Cannot evaluate genvar cas | r 200 bits=8 | def 9 bits=4 | def 9 bits=4 | same (no oracle) |
| V31_widefwd | ERR:V31_widefwd.sv:4: error: Unable to bind parameter `K' in | k 200 bits=8 | k 200 bits=8 | k 8 bits=4 | k 200 bits=8 | silent→correct =sv+vl |
| V32_typefwd | def 9 bits=4 | t 200 bits=8 | ERR:%Error: V32_typefwd.sv:4:13: Reference to 'T' before dec | def 9 bits=4 | def 9 bits=4 | same |
| W1_wide_sh | def 9 bits=4 K=10000000000000005 | k 200 bits=8 K=05 | k 200 bits=8 K=05 | k 8 bits=4 K=05 | k 200 bits=8 K=05 | silent→correct =sv+vl |
| W2_narrow_over_wide_fwd | K=05 | K=10000000000000007 | K=10000000000000007 | K=10000000000000007 | K=10000000000000007 | same |
