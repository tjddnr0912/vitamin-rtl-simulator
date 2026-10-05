`timescale 1ns/1ns
module t;
  logic [3:0] v; parameter P = 4'b1x00;
  initial begin
    v=4'b1100; $display("E22 %b", v inside {P});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
