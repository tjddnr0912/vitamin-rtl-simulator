`timescale 1ns/1ns
module t;
  logic [3:0] v; localparam L = 4'b1?00;
  initial begin
    v=4'b1100; $display("E26 %b", v ==? L);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
