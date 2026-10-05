`timescale 1ns/1ns
module late;
  logic [7:0] u8 = 8'b0101_0100; logic signed [7:0] s8 = -8'sd4; logic signed [3:0] s4 = -4'sd4;
  logic [7:0] ux = 8'b0x01_x100;
endmodule
module t;
  logic [7:0] u8n;
  late uL();
  wire a1 = t.uL.u8 inside {4'b?100};
  wire a2 = t.u8n inside {4'b?100};
  initial begin
    u8n = 8'b0000_0100; #1;
    $display("cont %b %b", a1, a2);
    $display("hier %b %b %b", t.uL.u8 inside {4'b?100}, t.u8n inside {4'b?100}, t.uL.u8 inside {8'b0101_0100});
    $display("signed %b %b %b %b", t.uL.s8 inside {4'sb?100}, t.uL.s8 inside {4'sb1?00}, t.uL.s4 inside {8'sb1111_1?00}, t.uL.s4 inside {8'b1111_1?00});
    $display("xz %b %b %b", t.uL.ux inside {8'b0?01_?100}, t.uL.ux inside {8'b0101_?100}, t.uL.ux inside {8'b0111_?100});
    $display("weq %b %b", t.uL.u8 ==? 4'b?100, t.uL.s8 !=? 4'sb1?00);
    $finish;
  end
endmodule
