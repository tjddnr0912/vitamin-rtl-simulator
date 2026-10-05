`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x; reg [3:0] \inside ; int m1;
  initial begin
    \inside = 4'b0110; x = 4'd3;
    case (x) \inside [2:1]: m1 = 1; default: m1 = 0; endcase
    $display("esc m1=%0d", m1);
    $finish;
  end
endmodule
