`ifdef IV
 `define IN(a,b)  ((a) ==? b)
 `define NIN(a,b) ((a) !=? b)
`else
 `define IN(a,b)  ((a) inside {b})
 `define NIN(a,b) (!((a) inside {b}))
`endif
module t;
  logic u1; logic signed [0:0] s1;
  logic [30:0] u31; logic [31:0] u32; logic [32:0] u33, u33b; logic signed [32:0] s33;
  logic [62:0] u63; logic [63:0] u64; logic [64:0] u65, u65b; logic signed [64:0] s65, s65p;
  logic [126:0] u127; logic [127:0] u128; logic [128:0] u129, u129b, u129c; logic signed [128:0] s129, s129p;
  logic signed [69:0] s70; logic signed [3:0] s4;
  initial #1000 $finish;
  initial begin
    u1 = 1; s1 = 1'sb1; s4 = -4;
    u31 = 31'h4000_0004; u32 = 32'hF000_0004; u33 = 33'h1_0000_0004; u33b = 33'h1_0000_0001; s33 = 33'sh1_0000_0004;
    u63 = 63'h4000_0000_0000_0001; u64 = 64'hF000_0000_0000_0001; u65 = {1'b1, 64'h1}; u65b = 65'h3;
    s65 = -4; s65p = 65'sh54;
    u127 = {127{1'b1}}; u128 = {128{1'b1}}; u129 = {1'b1, 128'h4}; u129b = {1'b0, 128'h4}; u129c = {1'b1, 128'h5};
    s129 = -4; s129p = 129'sh54; s70 = -4;
    #1;
    $display("D01 %b", `IN(u1, 1'b?));
    $display("D02 %b", `IN(u1, 1'bx));
    $display("D03 %b", `IN(s1, 4'sb111?));
    $display("D04 %b", `IN(s1, 2'sb?1));
    $display("D05 %b", `IN(u33, 'b?100));
    $display("D06 %b", `IN(u33b, 'bz1));
    $display("D07 %b", `IN(u64, 'bx1));
    $display("D08 %b", `IN(u65, 'bx1));
    $display("D09 %b", `IN(u65, 'h?));
    $display("D10 %b", `IN(u65, 'dx));
    $display("D10b %b", `IN(u65, 'd?));
    $display("D11 %b", `IN(u65, 'b1x));
    $display("D12 %b", `IN(u65b, 'b1x));
    $display("D13 %b", `IN(u129, 129'h?_0000_0000_0000_0000_0000_0000_0000_0004));
    $display("D14 %b", `IN(u129b, 129'h?_0000_0000_0000_0000_0000_0000_0000_0004));
    $display("D15 %b", `IN(u129c, 129'h?_0000_0000_0000_0000_0000_0000_0000_0004));
    $display("D16 %b", `IN(u65, 65'h?_0000_0000_0000_0001));
    $display("D17 %b", `IN(s65, 4'sb1?00));
    $display("D18 %b", `IN(s65p, 4'sb?100));
    $display("D19 %b", `IN(s4, 65'sh1_FFFF_FFFF_FFFF_FF?C));
    $display("D20 %b", `IN(s129, 4'sb1?00));
    $display("D21 %b", `IN(s129p, 4'sb?100));
    $display("D22 %b", `IN(s4, 129'sh1_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FF?C));
    $display("D23 %b", `IN(s70, 65'sh1_FFFF_FFFF_FFFF_FF?C));
    $display("D24 %b", `IN(u33, 32'sh?000_0004));
    $display("D25 %b", `IN(s33, 32'sh?000_0004));
    $display("D26 %b", `IN(u128, 128'h?FFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF));
    $display("D27 %b", `IN(u127, 'bx));
    $display("D28 %b", `IN(u65, 'x));
    $display("D29 %b", `IN(u65, 'z));
    $display("D30 %b", `IN(u31, 'b?));
    $display("D31 %b", `IN(u32, 32'b????_0000_0000_0000_0000_0000_0000_0100));
    $display("D32 %b", `IN(u63, 64'h?000_0000_0000_0001));
    $display("D33 %b", `NIN(s65, 4'sb1?00));
    $display("D34 %b", `NIN(u65, 'bx1));
    $display("D35 %b", `IN(s129, 129'sh1_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FF?C));
    $display("D36 %b", `IN(u65, 'sbx1));
    $display("D37 %b", `IN(s65, 'sbx0));
    $display("D38 %b", `IN(s65, 'sb1x00));
    $display("D39 %b", `IN(s4, 'sbx100));
    $display("D40 %b", `IN(s1, 'sb?));
  end
endmodule
