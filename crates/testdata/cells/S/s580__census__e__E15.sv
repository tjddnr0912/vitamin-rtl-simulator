`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b0000; $display("E15 %b", v inside {4'b0001, 'x});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
