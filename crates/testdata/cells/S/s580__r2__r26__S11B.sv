`timescale 1ns/1ns
module t;
  logic [((4'd15 + 4'd1) inside {5'b1?000}):0] wb_S11;
  initial begin #1 $display("S11B %0d", $bits(wb_S11)); #1 $finish; end
  initial #100 $finish;
endmodule
