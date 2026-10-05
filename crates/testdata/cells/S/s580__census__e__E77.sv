`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [7:0] w8;
  initial begin
    v=4'b1100; w8 = v inside {4'b1?00}; $display("E77 %b", w8);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
