`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("E45 %b", v inside {{2{2'b?0}}});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
