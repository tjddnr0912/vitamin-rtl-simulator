`timescale 1ns/1ns
module t;
  logic [7:0] a; logic [3:0] b;
  initial begin
    a = 8'd1; b = 4'b1000;
    $display("sc %b %b %b", 8'(a + (b ==? 4'b1x0x)), 8'(a + (b !=? 4'b1x0x)), 8'(a + (b inside {4'b1x0x})));
    $finish;
  end
endmodule
