`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("E40 %b", v inside {{2'b1?, 2'b00}});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
