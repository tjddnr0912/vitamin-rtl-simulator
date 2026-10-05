`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("E04 %b", v inside {'b?});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
