`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b0010; $display("E03 %b", v inside {'bx1});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
