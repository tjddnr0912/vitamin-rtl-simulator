`ifdef IV
 `define IN(a,b)  ((a) ==? b)
 `define IN3(a,p1,lo,hi,p2) ((((a) ==? p1)) || (((a) >= (lo)) && ((a) <= (hi))) || (((a) ==? p2)))
`else
 `define IN(a,b)  ((a) inside {b})
 `define IN3(a,p1,lo,hi,p2) ((a) inside {p1, [lo:hi], p2})
`endif
`define XA (4'b111x | 4'b0001)
`define XB (4'b000x | 4'b0001)
`define XS (4'sb111x & 4'sb1110)
module t;
  localparam [3:0] A4 = 4'd15, B4 = 4'd1;
  localparam signed [3:0] SA = -4'sd2;
  localparam [63:0] F64 = 64'hFFFF_FFFF_FFFF_FFFF;
  localparam [62:0] F63 = 63'h7FFF_FFFF_FFFF_FFFF;
  localparam [127:0] F128 = {128{1'b1}};
  localparam [63:0] H64 = 64'h8000_0000_0000_0000;
  localparam C1 = 1'b1;
  localparam L01 = `IN(A4 + B4, 5'b1?000);
  localparam L02 = `IN(B4 - A4, 5'b1?010);
  localparam L03 = `IN(A4 << 1, 5'b1111?);
  localparam L04 = `IN(~B4, 8'b1111_111?);
  localparam L05 = `IN(-B4, 8'b1111_111?);
  localparam L06 = `IN({A4, B4}, 9'b1_1111_000?);
  localparam L07 = `IN({2{B4}}, 9'b0_0001_000?);
  localparam L08 = `IN(C1 ? A4 + B4 : B4, 5'b1?000);
  localparam L09 = `IN($signed(B4 - A4), 5'b1?010);
  localparam L10 = `IN($signed(B4 - A4), 5'sb0?010);
  localparam L11 = `IN($unsigned(SA), 8'b0000_111?);
  localparam L12 = `IN(4'(A4 + B4), 5'b0?000);
  localparam L13 = `IN($clog2(17), 'b1?1);
  localparam L14 = `IN($bits(A4), 3'b1?0);
  localparam L15 = `IN(F64 + 64'd1, 65'h1_0000_0000_0000_000?);
  localparam L16 = `IN(F63 + 63'd1, 64'h8000_0000_0000_000?);
  localparam L17 = `IN(F128 + 128'd1, 129'h1_0000_0000_0000_0000_0000_0000_0000_000?);
  localparam L18 = `IN(H64 << 1, 65'h1_0000_0000_0000_000?);
  localparam L19 = `IN(SA, 8'sb1111_11?0);
  localparam L20 = `IN(SA, 8'b1111_11?0);
  localparam L21 = `IN(SA + 4'sd0, 8'sb1111_11?0);
  localparam L22 = `IN(SA >>> 1, 8'sb1111_11?1);
  localparam L23 = `IN(SA >>> 1, 8'b0000_011?);
  localparam L24 = `IN(-SA, 8'sb0000_001?);
  localparam L25 = `IN(B4 - A4, 'b1?010);
  localparam L26 = `IN(B4 - A4, 'bx_1?010);
  localparam L27 = ((A4 + B4) !=? 5'b1?000);
  localparam L28 = ((B4 - A4) !=? 5'b0?010);
  localparam L29 = `IN(F64 + 64'd1, 65'h0_FFFF_FFFF_FFFF_FFF?);
  localparam L30 = `IN(F63 << 1, 64'hFFFF_FFFF_FFFF_FFF?);
  localparam signed [64:0] S65 = -65'sd2;
  localparam [64:0] B65 = 65'd1;
  localparam signed [63:0] S64 = -64'sd2;
  localparam L31 = `IN(S65, 66'sh3_FFFF_FFFF_FFFF_FFF?);
  localparam L32 = `IN(S65, 66'h3_FFFF_FFFF_FFFF_FFF?);
  localparam L33 = `IN(S65 >>> 1, 66'h0_FFFF_FFFF_FFFF_FFF?);
  localparam L34 = `IN(S65 >>> 1, 66'sh3_FFFF_FFFF_FFFF_FFF?);
  localparam L35 = `IN(-S65, 66'sh0_0000_0000_0000_000?);
  localparam L36 = `IN(-B65, 66'h3_FFFF_FFFF_FFFF_FFF?);
  localparam L37 = `IN(~B65, 66'h3_FFFF_FFFF_FFFF_FFF?);
  localparam L38 = `IN(S64 >>> 1, 64'hFFFF_FFFF_FFFF_FFF?);
  localparam L39 = `IN(SA >>> 1, 4'b011?);
  localparam L40 = `IN(S65 >>> 1, 65'h0_FFFF_FFFF_FFFF_FFF?);
  localparam L41 = `IN(SA >>> 1, 4'sb111?);
  localparam L42 = `IN(S64 >>> 1, 64'shFFFF_FFFF_FFFF_FFF?);
  localparam M01 = `IN3(A4 + B4, 5'b0?001, 5'd17, 5'd20, 5'b1?000);
  localparam M02 = `IN3(A4 + B4, 5'b0?001, 5'd17, 5'd20, 4'b0000);
  localparam M03 = `IN3(F64 + 64'd1, 65'h0_0000_0000_0000_000?, 65'h1_0000_0000_0000_0000, 65'h1_0000_0000_0000_0001, 64'h0);
`ifdef WX
  localparam W01 = `IN(`XA + `XB, 5'b1?000);
`endif
`ifdef WX
  localparam W02 = `IN(`XB - `XA, 5'b1?010);
`endif
`ifdef WX
  localparam W04 = `IN(~`XB, 8'b1111_111?);
`endif
`ifdef WX
  localparam W05 = `IN(-`XB, 8'b1111_111?);
`endif
`ifdef WX
  localparam W06 = `IN({`XA, `XB}, 9'b1_1111_000?);
`endif
`ifdef WX
  localparam W08 = `IN(C1 ? `XA + `XB : `XB, 5'b1?000);
`endif
`ifdef WX
  localparam W09 = `IN($signed(`XB - `XA), 5'b1?010);
`endif
`ifdef WX
  localparam W10 = `IN($signed(`XB - `XA), 5'sb0?010);
`endif
`ifdef WX
  localparam W12 = `IN(4'(`XA + `XB), 5'b0?000);
`endif
`ifdef WX
  localparam W19 = `IN(`XS, 8'sb1111_11?0);
`endif
`ifdef WX
  localparam W20 = `IN(`XS, 8'b1111_11?0);
`endif
`ifdef WX
  localparam W22 = `IN(`XS >>> 1, 8'sb1111_11?1);
`endif
`ifdef WX
  localparam W23 = `IN(`XS >>> 1, 8'b0000_011?);
`endif
`ifdef WX
  localparam W24 = `IN(-`XS, 8'sb0000_001?);
`endif
`ifdef WX
  localparam W25 = `IN(`XB - `XA, 'b1?010);
`endif
`ifdef WX
  localparam W26 = `IN(`XB - `XA, 'bx_1?010);
`endif
`ifdef WX
  localparam W27 = ((`XA + `XB) !=? 5'b1?000);
`endif
`ifdef WX
  localparam W31 = `IN(`XA + `XB, 'x);
`endif
`ifdef WX
  localparam W32 = `IN(`XB - `XA, 5'b1?01x);
`endif
  logic [3:0] a4, b4; logic signed [3:0] sa; logic [63:0] f64, h64; logic [62:0] f63; logic [127:0] f128; logic c1; int i17; logic signed [64:0] s65; logic [64:0] b65; logic signed [63:0] s64;
  initial begin
    a4 = 15; b4 = 1; sa = -2; f64 = '1; f63 = '1; f128 = '1; h64 = 64'h8000_0000_0000_0000; c1 = 1; i17 = 17; s65 = -65'sd2; b65 = 65'd1; s64 = -64'sd2;
    #1;
    $display("L01 %b", L01); $display("R01 %b", `IN(a4 + b4, 5'b1?000));
    $display("L02 %b", L02); $display("R02 %b", `IN(b4 - a4, 5'b1?010));
    $display("L03 %b", L03); $display("R03 %b", `IN(a4 << 1, 5'b1111?));
    $display("L04 %b", L04); $display("R04 %b", `IN(~b4, 8'b1111_111?));
    $display("L05 %b", L05); $display("R05 %b", `IN(-b4, 8'b1111_111?));
    $display("L06 %b", L06); $display("R06 %b", `IN({a4, b4}, 9'b1_1111_000?));
    $display("L07 %b", L07); $display("R07 %b", `IN({2{b4}}, 9'b0_0001_000?));
    $display("L08 %b", L08); $display("R08 %b", `IN(c1 ? a4 + b4 : b4, 5'b1?000));
    $display("L09 %b", L09); $display("R09 %b", `IN($signed(b4 - a4), 5'b1?010));
    $display("L10 %b", L10); $display("R10 %b", `IN($signed(b4 - a4), 5'sb0?010));
    $display("L11 %b", L11); $display("R11 %b", `IN($unsigned(sa), 8'b0000_111?));
    $display("L12 %b", L12); $display("R12 %b", `IN(4'(a4 + b4), 5'b0?000));
    $display("L13 %b", L13); $display("R13 %b", `IN($clog2(i17), 'b1?1));
    $display("L14 %b", L14); $display("R14 %b", `IN($bits(a4), 3'b1?0));
    $display("L15 %b", L15); $display("R15 %b", `IN(f64 + 64'd1, 65'h1_0000_0000_0000_000?));
    $display("L16 %b", L16); $display("R16 %b", `IN(f63 + 63'd1, 64'h8000_0000_0000_000?));
    $display("L17 %b", L17); $display("R17 %b", `IN(f128 + 128'd1, 129'h1_0000_0000_0000_0000_0000_0000_0000_000?));
    $display("L18 %b", L18); $display("R18 %b", `IN(h64 << 1, 65'h1_0000_0000_0000_000?));
    $display("L19 %b", L19); $display("R19 %b", `IN(sa, 8'sb1111_11?0));
    $display("L20 %b", L20); $display("R20 %b", `IN(sa, 8'b1111_11?0));
    $display("L21 %b", L21); $display("R21 %b", `IN(sa + 4'sd0, 8'sb1111_11?0));
    $display("L22 %b", L22); $display("R22 %b", `IN(sa >>> 1, 8'sb1111_11?1));
    $display("L23 %b", L23); $display("R23 %b", `IN(sa >>> 1, 8'b0000_011?));
    $display("L24 %b", L24); $display("R24 %b", `IN(-sa, 8'sb0000_001?));
    $display("L25 %b", L25); $display("R25 %b", `IN(b4 - a4, 'b1?010));
    $display("L26 %b", L26); $display("R26 %b", `IN(b4 - a4, 'bx_1?010));
    $display("L27 %b", L27); $display("R27 %b", ((a4 + b4) !=? 5'b1?000));
    $display("L28 %b", L28); $display("R28 %b", ((b4 - a4) !=? 5'b0?010));
    $display("L29 %b", L29); $display("R29 %b", `IN(f64 + 64'd1, 65'h0_FFFF_FFFF_FFFF_FFF?));
    $display("L30 %b", L30); $display("R30 %b", `IN(f63 << 1, 64'hFFFF_FFFF_FFFF_FFF?));
    $display("L31 %b", L31); $display("R31 %b", `IN(s65, 66'sh3_FFFF_FFFF_FFFF_FFF?));
    $display("L32 %b", L32); $display("R32 %b", `IN(s65, 66'h3_FFFF_FFFF_FFFF_FFF?));
    $display("L33 %b", L33); $display("R33 %b", `IN(s65 >>> 1, 66'h0_FFFF_FFFF_FFFF_FFF?));
    $display("L34 %b", L34); $display("R34 %b", `IN(s65 >>> 1, 66'sh3_FFFF_FFFF_FFFF_FFF?));
    $display("L35 %b", L35); $display("R35 %b", `IN(-s65, 66'sh0_0000_0000_0000_000?));
    $display("L36 %b", L36); $display("R36 %b", `IN(-b65, 66'h3_FFFF_FFFF_FFFF_FFF?));
    $display("L37 %b", L37); $display("R37 %b", `IN(~b65, 66'h3_FFFF_FFFF_FFFF_FFF?));
    $display("L38 %b", L38); $display("R38 %b", `IN(s64 >>> 1, 64'hFFFF_FFFF_FFFF_FFF?));
    $display("L39 %b", L39); $display("R39 %b", `IN(sa >>> 1, 4'b011?));
    $display("L40 %b", L40); $display("R40 %b", `IN(s65 >>> 1, 65'h0_FFFF_FFFF_FFFF_FFF?));
    $display("L41 %b", L41); $display("R41 %b", `IN(sa >>> 1, 4'sb111?));
    $display("L42 %b", L42); $display("R42 %b", `IN(s64 >>> 1, 64'shFFFF_FFFF_FFFF_FFF?));
    $display("M01 %b", M01); $display("RM01 %b", `IN3(a4 + b4, 5'b0?001, 5'd17, 5'd20, 5'b1?000));
    $display("M02 %b", M02); $display("RM02 %b", `IN3(a4 + b4, 5'b0?001, 5'd17, 5'd20, 4'b0000));
    $display("M03 %b", M03); $display("RM03 %b", `IN3(f64 + 64'd1, 65'h0_0000_0000_0000_000?, 65'h1_0000_0000_0000_0000, 65'h1_0000_0000_0000_0001, 64'h0));
`ifdef WX
    $display("W01 %b", W01); $display("W02 %b", W02); $display("W04 %b", W04); $display("W05 %b", W05);
`endif
`ifdef WX
    $display("W06 %b", W06); $display("W08 %b", W08); $display("W09 %b", W09); $display("W10 %b", W10);
`endif
`ifdef WX
    $display("W12 %b", W12); $display("W19 %b", W19); $display("W20 %b", W20); $display("W22 %b", W22);
`endif
`ifdef WX
    $display("W23 %b", W23); $display("W24 %b", W24); $display("W25 %b", W25); $display("W26 %b", W26);
`endif
`ifdef WX
    $display("W27 %b", W27); $display("W31 %b", W31); $display("W32 %b", W32);
`endif
    #1 $finish;
  end
endmodule
