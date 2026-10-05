`timescale 1ns/1ns
module t;
  logic [35:0] v;
  initial begin
    v=36'hF_0000_0001; $display("E05 %b", v inside {'bx1});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
