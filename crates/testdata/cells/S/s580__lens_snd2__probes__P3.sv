`timescale 1ns/1ns
module late;
  logic signed [3:0] s4 = 4'sb1000;
  logic signed [7:0] s8 = 8'sb1111_1100;
  logic [7:0] u8 = 8'b1111_1100;
  logic [3:0] u4 = 4'b1100;
  logic signed [7:0] sx = 8'sbx111_1100;
  logic signed [3:0] s4x = 4'sbx100;
  logic signed [67:0] s68 = -68'sd4;
endmodule
module t;
  late uL();
  initial begin
    #1;
    $display("H1 %b %b %b %b", t.uL.s4 ==? 8'sb1111_1?00, t.uL.s4 ==? 8'sb0000_1?00, t.uL.u4 ==? 8'sb1111_1?00, t.uL.u4 ==? 8'sb0000_1?00);
    $display("H2 %b %b %b %b", t.uL.s8 ==? 4'sb1?00, t.uL.s8 ==? 4'sb?100, t.uL.u8 ==? 4'sb1?00, t.uL.u8 ==? 4'sb?100);
    $display("H3 %b %b %b %b", t.uL.sx ==? 4'sb1?00, t.uL.sx ==? 4'sb?100, t.uL.s4x ==? 8'sb1111_?100, t.uL.s4x ==? 8'sb????_?100);
    $display("H4 %b %b %b", t.uL.s68 ==? 4'sb1?00, t.uL.s68 ==? 4'sb?100, t.uL.s68 !=? 4'sb0?00);
    $display("H5 %b %b %b", (t.uL.s4 + 8'sd0) ==? 8'sb1111_1?00, (t.uL.s4 + 8'd0) ==? 8'sb1111_1?00, (t.uL.s8 >>> 1) ==? 4'sb1?10);
`ifndef NO_INSIDE
    $display("I1 %b %b %b %b", t.uL.s4 inside {8'sb1111_1?00}, t.uL.s8 inside {4'sb?100}, t.uL.sx inside {4'sb1?00}, t.uL.s68 inside {4'sb1?00});
`endif
    #1 $finish;
  end
endmodule
