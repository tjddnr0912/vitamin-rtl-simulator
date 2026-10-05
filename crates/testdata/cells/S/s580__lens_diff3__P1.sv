`ifdef IV
 `define IN(a,b)  ((a) ==? b)
 `define IN2(a,b,c) (((a) ==? b) || ((a) ==? c))
`else
 `define IN(a,b)  ((a) inside {b})
 `define IN2(a,b,c) ((a) inside {b, c})
`endif
module t;
  logic signed [3:0] s4; logic signed [7:0] s8, s8b, s8z, s8o; logic [7:0] u8; logic c;
  int i; byte b; shortint sh; longint l; integer ig; bit signed [3:0] bs4;
  logic signed [7:0] arr [0:3];
  typedef struct packed { logic signed [3:0] a; logic [3:0] b; } st_t;
  typedef struct packed signed { logic [3:0] a; logic [3:0] b; } sst_t;
  st_t st; sst_t sst;
  function automatic logic signed [7:0] fs(input logic signed [7:0] x); return x; endfunction
  function automatic logic fwq(input logic signed [7:0] x); return ((x >>> 1) ==? 8'sb1100_000?); endfunction
  function automatic logic fin(input logic signed [7:0] x); return `IN(x >>> 1, 8'sb1100_000?); endfunction
  wire w1 = ((s8 >>> 1) ==? 8'sb1100_000?);
  wire w2 = `IN(b >>> 1, 8'sb1100_000?);
  logic ac1, ac2;
  always_comb ac1 = ((st.a >>> 1) ==? 4'sb111?);
  always_comb ac2 = `IN(c ? s4 : s8, 8'sb1111_1?00);
`ifdef CONSTS
  localparam signed [7:0] S8 = -8'sd128; localparam signed [3:0] S4 = -4'sd4; localparam [7:0] U8 = 8'h80;
  localparam int I = -6;
  localparam K01 = ((S8 >>> 1) ==? 8'sb1100_000?);
  localparam K02 = `IN(S8 >>> 1, 8'sb1100_000?);
  localparam K11 = (($signed(U8) >>> 1) ==? 8'sb1100_000?);
  localparam K12 = (($unsigned(S8) >>> 1) ==? 8'sb1100_000?);
  localparam K13 = ((S8[7:0] >>> 1) ==? 8'sb1100_000?);
  localparam K14 = (({S4, S4} >>> 1) ==? 8'sb1110_011?);
  localparam K16 = ((1'b1 ? S4 : S8) ==? 8'sb1111_1?00);
  localparam K18 = (((-S4) + 8'sd0) ==? 8'sb0000_01??);
  localparam K21 = ((~S4 + 8'sd0) ==? 8'sb0000_00??);
  localparam K23 = ((I >>> 1) ==? 'sb0?11_1111_1111_1111_1111_1111_1111_1101);
  localparam K24 = ((S8 >>> 2) ==? 8'sb?110_0000);
  localparam K27 = `IN2(S8 >>> 2, 8'sb0110_000?, 8'sb1110_000?);
  localparam K28 = `IN2(S8 >>> 2, 8'b1110_000?, 8'sb0010_000?);
  localparam K39 = ((S8 > 0 ? S8 : S4) ==? 8'sb1111_1?00);
`endif
`ifdef KF
  localparam KF1 = fwq(-8'sd128);
  localparam KF2 = fin(-8'sd128);
`endif
  initial begin
    s4 = -4; s8 = -128; s8b = -6; s8z = 0; s8o = 1; u8 = 8'h80; c = 1;
    i = -6; b = -128; sh = -7; l = -256; ig = -6; bs4 = -2;
    arr[0] = 0; arr[1] = -128; arr[2] = 0; arr[3] = 0; st = 8'b1110_0000; sst = 8'b1000_0000;
    #1;
    $display("B01 %b", ((b >>> 1) ==? 8'sb1100_000?));
    $display("B02 %b", ((sh >>> 1) ==? 16'sb1111_1111_1111_110?));
    $display("B03 %b", ((i >>> 1) ==? 32'shFFFF_FFF?));
    $display("B04 %b", ((l >>> 4) ==? 64'shFFFF_FFFF_FFFF_FFF?));
    $display("B05 %b", ((ig / 2) ==? 32'shFFFF_FFF?));
    $display("B06 %b", ((bs4 >>> 1) ==? 4'sb111?));
    $display("B07 %b", ((arr[1] >>> 1) ==? 8'sb1100_000?));
    $display("B08 %b", ((st.a >>> 1) ==? 4'sb111?));
    $display("B09 %b", ((sst >>> 1) ==? 8'sb1100_000?));
    $display("B10 %b", ((st >>> 1) ==? 8'sb1111_000?));
    $display("B11 %b", (($signed(u8) >>> 1) ==? 8'sb1100_000?));
    $display("B12 %b", (($unsigned(s8) >>> 1) ==? 8'sb1100_000?));
    $display("B13 %b", ((s8[7:0] >>> 1) ==? 8'sb1100_000?));
    $display("B14 %b", (({s4, s4} >>> 1) ==? 8'sb1110_011?));
    $display("B15 %b", (((c ? s8 : u8) >>> 1) ==? 8'sb1100_000?));
    $display("B16 %b", ((c ? s4 : s8) ==? 8'sb1111_1?00));
    $display("B17 %b", ((fs(s8) >>> 1) ==? 8'sb1100_000?));
    $display("B18 %b", (((-s4) + s8z) ==? 8'sb0000_01??));
    $display("B19 %b", ((s4 * s8o) ==? 8'sb1111_11??));
    $display("B20 %b", (((s4 <<< 1) + s8z) ==? 8'sb1111_10??));
    $display("B21 %b", ((~s4 + s8z) ==? 8'sb0000_00??));
    $display("B22 %b", `IN((s4 * s8o), 8'sb1111_11??));
    $display("B23 %b", ((i >>> 1) ==? 'sb0?11_1111_1111_1111_1111_1111_1111_1101));
    $display("B24 %b", ((s8 >>> 2) ==? 8'sb?110_0000));
    $display("B25 %b", ((s8 >>> 2) ==? 8'sb11z0_0000));
    $display("B26 %b", ((s8 >>> 2) !=? 8'sb?110_0000));
    $display("B27 %b", `IN2(s8 >>> 2, 8'sb0110_000?, 8'sb1110_000?));
    $display("B28 %b", `IN2(s8 >>> 2, 8'b1110_000?, 8'sb0010_000?));
    $display("B29 %b", (((s8 >>> 2) ==? 8'b1110_000?) || ((s8 >>> 2) ==? 8'sb0010_000?)));
    $display("B30 %b", ((u8 >>> 1) ==? 8'sb1100_000?));
    $display("B31 %b", ((s8 >>> 1) ==? 8'b1100_000?));
    $display("B32 %b", w1); $display("B33 %b", w2); $display("B34 %b", ac1); $display("B35 %b", ac2);
    $display("B36 %b", fwq(s8)); $display("B37 %b", fin(s8));
    $display("B38 %b", (($signed({s4, s4}) >>> 1) ==? 8'sb1110_011?));
    $display("B39 %b", (((s8 > 0) ? s8 : s4) ==? 8'sb1111_1?00));
    $display("B40 %b", `IN(s4 + s8z, 'sb1?00));
`ifdef CONSTS
    $display("K01 %b", K01); $display("K02 %b", K02); $display("K11 %b", K11); $display("K12 %b", K12);
    $display("K13 %b", K13); $display("K14 %b", K14); $display("K16 %b", K16); $display("K18 %b", K18);
    $display("K21 %b", K21); $display("K23 %b", K23); $display("K24 %b", K24); $display("K27 %b", K27);
    $display("K28 %b", K28); $display("K39 %b", K39);
`endif
`ifdef KF
    $display("KF1 %b", KF1); $display("KF2 %b", KF2);
`endif
    #1 $finish;
  end
endmodule
