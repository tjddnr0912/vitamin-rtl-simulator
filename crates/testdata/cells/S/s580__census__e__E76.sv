`timescale 1ns/1ns
module t;
  logic [3:0] v; bit b;
  initial begin
    v=4'b1100; b = v inside {4'b1?00}; $display("E76 %b", b);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
