`timescale 1ns/1ns
module t;
  localparam logic [3:0] PV = 4'b1100;
  if (PV inside {4'b1?00}) begin : g initial $display("C13 then"); end
  else begin : g2 initial $display("C13 else"); end
  initial #1 $finish;
  initial #100 $finish;
endmodule
