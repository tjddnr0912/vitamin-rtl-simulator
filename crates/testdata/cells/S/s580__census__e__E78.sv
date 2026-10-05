`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("E78 %b", v[3:0] inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
