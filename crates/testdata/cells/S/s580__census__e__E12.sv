`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1111; $display("E12 %b", v inside {'1});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
