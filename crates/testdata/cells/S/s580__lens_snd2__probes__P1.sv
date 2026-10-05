`timescale 1ns/1ns
module late;
  logic [35:0] v36 = 36'hF_0000_0000;
  logic [35:0] w36 = 36'hF_FFFF_FFFF;
  logic [7:0]  v8  = 8'hFF;
  logic signed [7:0] s8 = -8'sd1;
endmodule
module t;
  late uL();
  logic [35:0] lv36 = 36'hF_0000_0000;
  logic [7:0] lv8 = 8'hFF;
  initial begin
    #1;
`ifndef ONLYC
    $display("F1 %b %b %b", t.uL.v36 ==? 'x, t.uL.v36 ==? 'z, t.uL.v36 !=? 'x);
    $display("F2 %b %b %b", t.uL.w36 ==? '1, t.uL.v8 ==? '1, t.uL.s8 ==? '1);
    $display("F3 %b %b", t.uL.v8 ==? 'x, t.uL.v8 ==? '0);
    $display("L1 %b %b %b", lv36 ==? 'x, lv8 ==? '1, lv36 !=? 'x);
`endif
`ifndef NO_INSIDE
    $display("I1 %b %b %b", t.uL.v36 inside {'x}, t.uL.v36 inside {'z}, t.uL.w36 inside {'1});
    $display("I2 %b %b", t.uL.v8 inside {'1}, t.uL.v8 inside {'x});
`endif
    $display("C1 %b %b", t.uL.w36 == '1, t.uL.v8 == '1);
    #1 $finish;
  end
endmodule
