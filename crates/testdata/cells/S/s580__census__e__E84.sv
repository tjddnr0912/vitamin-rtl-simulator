`timescale 1ns/1ns
module t;
  logic signed [3:0] v;
  initial begin
    v=4'sb1100; $display("E84 %b", v inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
