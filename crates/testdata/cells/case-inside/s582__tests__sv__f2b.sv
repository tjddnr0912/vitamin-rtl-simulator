`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] x; int m;
  initial begin
    x = 7;
    for (int inside = 6; inside < 7; inside++) begin case (x) inside + 1: m = 1; default: m = 0; endcase end
    $display("forvar m=%0d", m);
    $finish;
  end
endmodule
