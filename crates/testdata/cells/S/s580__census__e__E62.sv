`timescale 1ns/1ns
module t;
  real r;
  initial begin
    r=1.5; $display("E62 %b", r inside {1.5});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
