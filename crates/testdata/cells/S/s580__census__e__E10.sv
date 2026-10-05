`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1010; $display("E10 %b", v inside {'x});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
