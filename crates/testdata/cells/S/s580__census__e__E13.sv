`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b0111; $display("E13 %b", v inside {'1});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
