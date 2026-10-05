`timescale 1ns/1ns
module t;
  bit [3:0] bv;
  initial begin
    bv=4'b1100; $display("E74 %b", bv inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
