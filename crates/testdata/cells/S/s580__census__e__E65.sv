`timescale 1ns/1ns
module t;
  real r;
  initial begin
    r=12.0; $display("E65 %b", r inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
