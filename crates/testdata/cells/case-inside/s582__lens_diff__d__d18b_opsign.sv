`timescale 1ns/1ns
module top;
  logic [7:0] e8; bit signed [3:0] s4; logic [31:0] e32;
  int m1, m2, m4, m5, m9, m9q;
  initial begin
    s4 = -4'sd1; e8 = 8'd16;
    case (e8) inside s4 + 1: m1 = 1; default: m1 = 0; endcase
    m2 = (e8 == s4 + 1);
    case (e8) inside [s4 + 1 : 8'd20]: m4 = 1; default: m4 = 0; endcase
    e32 = 32'd16;
    case (e32) inside s4 + 1: m5 = 1; default: m5 = 0; endcase
    e8 = 8'd7;
    case (e8) inside (s4 + 0) >>> 1: m9 = 1; default: m9 = 0; endcase
    m9q = (e8 == (s4 + 0) >>> 1);
    $display("m1=%0d m2=%0d m4=%0d m5=%0d m9=%0d m9q=%0d", m1, m2, m4, m5, m9, m9q);
    $finish;
  end
  initial #1000 $finish;
endmodule
