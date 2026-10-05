import os, sys
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen")
def case_text(sp, sel, arms, default, qual, ind="    "):
    q = (qual + " ") if qual else ""
    L = []
    if sp == "ci":
        L.append(f"{ind}{q}case ({sel}) inside")
        for items, st in arms:
            L.append(f"{ind}  {', '.join(items)}: {st}")
        if default is not None: L.append(f"{ind}  default: {default}")
        L.append(f"{ind}endcase")
    elif sp == "d":
        L.append(f"{ind}{q}case (1'b1)")
        for items, st in arms:
            L.append(f"{ind}  ({sel} inside {{{', '.join(items)}}}): {st}")
        if default is not None: L.append(f"{ind}  default: {default}")
        L.append(f"{ind}endcase")
    elif sp == "if":
        for i, (items, st) in enumerate(arms):
            kw = (q + "if") if i == 0 else "else if"
            L.append(f"{ind}{kw} ({sel} inside {{{', '.join(items)}}}) {st}")
        if default is not None: L.append(f"{ind}else {default}")
    return "\n".join(L)
def gen(name, decls, sel, arms, default, vals, disp, qual="", init="m = 9;"):
    for sp in ("ci", "d", "if"):
        body = []
        for v in vals:
            body.append(f"    {v}; {init}")
            body.append(case_text(sp, sel, arms, default, qual))
            body.append(f"    {disp}")
        src = f"module top;\n  {decls}\n  initial begin\n" + "\n".join(body) + "\n    #10 $finish;\n  end\nendmodule\n"
        open(os.path.join(OUT, f"{name}_{sp}.sv"), "w").write(src)
D4 = 'logic [3:0] v; int m;'
P4 = '$display("v=%b m=%0d", v, m);'
def vv(*xs): return [f"v = {x}" for x in xs]
gen("c01_repro", D4, "v", [(["4'b1?00"], "m = 1;"), (["[4'd1:4'd3]"], "m = 2;")], "m = 0;",
    vv("4'b1000","4'b0010","4'b0110","4'b1100","4'b0000","4'b0011","4'b0100","4'b0001"), P4)
gen("c02_xz_item", D4, "v", [(["4'b1x0z"], "m = 1;"), (["4'b0zz1"], "m = 2;"), (["4'b??11"], "m = 3;")], "m = 0;",
    vv("4'b1000","4'b1001","4'b1100","4'b1101","4'b0001","4'b0111","4'b0011","4'b0000","4'b1010","4'b1011"), P4)
gen("c03_xz_sel", D4, "v", [(["4'b1?00"], "m = 1;"), (["4'b0110"], "m = 2;"), (["[4'd8:4'd9]"], "m = 3;")], "m = 0;",
    vv("4'b1x00","4'bx100","4'b011x","4'b100z","4'bxxxx","4'b1z00","4'b1001","4'bzzzz","4'b10x1"), P4)
gen("c05_range_signed", 'logic signed [3:0] v; int m;', "v", [(["[-4'sd2:4'sd1]"], "m = 1;"), (["[4'sd3:4'sd5]"], "m = 2;")], "m = 0;",
    vv("-4'sd2","-4'sd1","4'sd0","4'sd1","4'sd2","4'sd3","-4'sd8","4'sd7"), '$display("v=%0d m=%0d", v, m);')
gen("c05b_range_mixed", 'logic signed [3:0] v; int m;', "v", [(["[4'd1:4'd3]"], "m = 1;"), (["[-1:1]"], "m = 2;")], "m = 0;",
    vv("-4'sd1","4'sd2","-4'sd7","4'sd0","4'sd1"), '$display("v=%0d m=%0d", v, m);')
gen("c05c_range_u_negbound", D4, "v", [(["[-1:4'sd2]"], "m = 1;"), (["[-4'sd2:4'sd2]"], "m = 2;")], "m = 0;",
    vv("4'd0","4'd1","4'd14","4'd15"), P4)
gen("c06a_narrow_sel_wide_item", D4, "v", [(["8'b0000_1?00"], "m = 1;"), (["8'h1F"], "m = 2;"), (["[8'd5:8'd6]"], "m = 3;"), (["8'b1???_??11"], "m = 4;")], "m = 0;",
    vv("4'b1100","4'b1000","4'b1111","4'd5","4'd6","4'b0011"), P4)
gen("c06b_wide_sel_narrow_item", 'logic [7:0] v; int m;', "v", [(["4'b1?00"], "m = 1;"), (["[4'd1:4'd2]"], "m = 2;")], "m = 0;",
    vv("8'h0C","8'h1C","8'h08","8'h01","8'h11","8'hF8"), '$display("v=%h m=%0d", v, m);')
gen("c06c_signed_wide_sel_signed_pat", 'logic signed [7:0] v; int m;', "v", [(["4'sb1?00"], "m = 1;"), (["4'b1?11"], "m = 2;")], "m = 0;",
    vv("8'shFC","8'shF8","8'sh0C","8'sh08","8'shFB","8'sh0B"), '$display("v=%h m=%0d", v, m);')
gen("c07_rev", D4, "v", [(["[4'd3:4'd1]"], "m = 1;")], "m = 0;", vv("4'd1","4'd2","4'd3"), P4)
gen("c08a_dollar_u", D4, "v", [(["[4'd12:$]"], "m = 1;"), (["[$:4'd2]"], "m = 2;")], "m = 0;", vv("4'd15","4'd12","4'd11","4'd0","4'd2","4'd3"), P4)
gen("c08b_dollar_s", 'logic signed [3:0] v; int m;', "v", [(["[$:-4'sd7]"], "m = 1;"), (["[4'sd6:$]"], "m = 2;")], "m = 0;",
    vv("-4'sd8","-4'sd7","-4'sd6","4'sd6","4'sd7","4'sd0"), '$display("v=%0d m=%0d", v, m);')
gen("c09_array", 'logic [3:0] v; int m; logic [3:0] arr [0:1] = \'{4\'d5, 4\'d7};', "v", [(["arr"], "m = 1;"), (["4'd2"], "m = 2;")], "m = 0;", vv("4'd5","4'd7","4'd2","4'd6"), P4)
gen("c10_multi", D4, "v", [(["4'd1", "4'd3", "[4'd8:4'd9]"], "m = 1;"), (["4'd2", "4'b11??"], "m = 2;")], "m = 0;", vv("4'd1","4'd3","4'd8","4'd9","4'd2","4'd12","4'd15","4'd4"), P4)
gen("c11_overlap", D4, "v", [(["[4'd0:4'd7]"], "m = 1;"), (["4'd5"], "m = 2;"), (["4'b01??"], "m = 3;"), (["4'b1???"], "m = 4;")], "m = 0;", vv("4'd5","4'd6","4'd8","4'd9"), P4)
gen("c12_nodef", D4, "v", [(["4'd1"], "m = 1;"), (["[4'd2:4'd3]"], "m = 2;")], None, vv("4'd1","4'd2","4'd4","4'bxx00"), P4)
gen("c13a_unique_nomatch", D4, "v", [(["4'd1"], "m = 1;"), (["[4'd2:4'd3]"], "m = 2;")], None, vv("4'd1","4'd5"), P4, qual="unique")
gen("c13b_unique_overlap", D4, "v", [(["[4'd0:4'd7]"], "m = 1;"), (["4'd5"], "m = 2;")], None, vv("4'd5","4'd1"), P4, qual="unique")
gen("c13c_unique0_nomatch", D4, "v", [(["4'd1"], "m = 1;"), (["[4'd2:4'd3]"], "m = 2;")], None, vv("4'd1","4'd5"), P4, qual="unique0")
gen("c13d_priority_nomatch", D4, "v", [(["4'd1"], "m = 1;"), (["[4'd2:4'd3]"], "m = 2;")], None, vv("4'd1","4'd5"), P4, qual="priority")
gen("c13e_priority_overlap", D4, "v", [(["[4'd0:4'd7]"], "m = 1;"), (["4'd5"], "m = 2;")], None, vv("4'd5"), P4, qual="priority")
gen("c13f_unique_default", D4, "v", [(["4'd1"], "m = 1;"), (["4'b1?00"], "m = 2;")], "m = 0;", vv("4'd1","4'd12","4'd3"), P4, qual="unique")
gen("c13g_unique_xsel", D4, "v", [(["4'd1"], "m = 1;"), (["4'b1?00"], "m = 2;")], None, vv("4'bx001","4'b1x00"), P4, qual="unique")
gen("c19a_sign_pairwise", 'logic signed [3:0] v; int m;', "v", [(["-1"], "m = 1;"), (["8'h00"], "m = 2;")], "m = 0;", vv("-4'sd1","4'sd0"), '$display("v=%0d m=%0d", v, m);')
gen("c19b_width_pairwise", 'logic [7:0] a, b; int m; logic [3:0] v;', "a + b", [(["8'h00"], "m = 1;"), (["16'h0100"], "m = 2;")], "m = 0;", ["a = 8'h80; b = 8'h80", "a = 8'h01; b = 8'hFF", "a = 8'h00; b = 8'h00"], '$display("a+b m=%0d", m);')
gen("c19c_sign_range_pairwise", 'logic signed [3:0] v; int m;', "v", [(["[-2:-1]"], "m = 1;"), (["[4'd0:4'd1]"], "m = 2;")], "m = 0;", vv("-4'sd1","-4'sd2","4'sd0"), '$display("v=%0d m=%0d", v, m);')
gen("c20a_const_sel", 'int m; logic [3:0] v;', "4'b1100", [(["4'b1?00"], "m = 1;"), (["[4'd1:4'd2]"], "m = 2;")], "m = 0;", ["v = 0"], '$display("m=%0d", m);')
gen("c20b_param_sel", 'int m; logic [3:0] v; localparam logic [3:0] P = 4\'b1000;', "P", [(["4'b1?00"], "m = 1;"), (["[4'd1:4'd2]"], "m = 2;")], "m = 0;", ["v = 0"], '$display("m=%0d", m);')
gen("c30_runtime_x_item", 'logic [3:0] v; int m; logic [3:0] w = 4\'b1x00;', "v", [(["w"], "m = 1;")], "m = 0;", vv("4'b1000","4'b1100","4'b0100"), P4)
gen("c31_compound_xz_item", D4, "v", [(["{2'b1?, 2'b00}"], "m = 1;")], "m = 0;", vv("4'b1000","4'b1100","4'b0100"), P4)
gen("c32_wide72", 'logic [71:0] v; int m;', "v", [(["72'h80_0000_0000_0000_00?0"], "m = 1;"), (["[72'h1:72'h3]"], "m = 2;")], "m = 0;", vv("72'h80_0000_0000_0000_0050","72'h80_0000_0000_0000_0051","72'h2","72'h0"), '$display("v=%h m=%0d", v, m);')
gen("c34_param_xz_item", 'logic [3:0] v; int m; localparam logic [3:0] P = 4\'b1?00;', "v", [(["P"], "m = 1;")], "m = 0;", vv("4'b1000","4'b1100","4'b0100"), P4)
gen("c37_allx_default", D4, "v", [(["4'b????"], "m = 1;")], "m = 0;", vv("4'bxxxx","4'bzzzz","4'd3"), P4)
gen("c38_empty_stmt", D4, "v", [(["4'd1"], ";"), (["4'd2"], "m = 2;")], "m = 0;", vv("4'd1","4'd2","4'd3"), P4)
gen("c39_unsized_items", D4, "v", [(["1", "3"], "m = 1;"), (["[4:7]"], "m = 2;"), (["'1"], "m = 3;")], "m = 0;", vv("4'd1","4'd3","4'd5","4'd15","4'd8"), P4)
gen("c40_fill_xz_item", 'logic [35:0] v; int m;', "v", [(["'bx1"], "m = 1;")], "m = 0;", vv("36'h1","36'h3","36'h2"), '$display("v=%h m=%0d", v, m);')
