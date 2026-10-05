`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, inside; reg [3:0] m1, m2;
  initial begin
    inside = 4'b0110; x = 4'd3;
    casez (x) inside[2:1]: m1 = 1; default: m1 = 0; endcase
    casex (x) inside[2:1]: m2 = 1; default: m2 = 0; endcase
    $display("F m1=%0d m2=%0d", m1, m2);
    $finish;
  end
endmodule
