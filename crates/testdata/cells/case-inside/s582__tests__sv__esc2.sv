`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, y; reg [3:0] \inside ; int m1, m2;
  initial begin
    \inside = 4'b0110; x = 4'd3; y = 4'd2;
    case (x) \inside [2:1]: m1 = 1; default: m1 = 0; endcase
    case (y) inside [4'd1:4'd3]: m2 = 1; default: m2 = 0; endcase
    $display("esc m1=%0d m2=%0d", m1, m2);
    $finish;
  end
endmodule
