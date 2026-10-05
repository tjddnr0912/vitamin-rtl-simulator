`timescale 1ns/1ns
module top;
  logic [15:0] e16; bit [7:0] a8, b8;
  int m1, m1q, m2, m2q, m3, m4;
  initial begin
    a8 = 8'hFF; b8 = 8'h01; e16 = 16'h0100;
    case (e16) inside 8'(a8 + b8): m1 = 1; default: m1 = 0; endcase
    m1q = (e16 == 8'(a8 + b8));
    case (e16) inside {a8 + b8}: m2 = 1; default: m2 = 0; endcase
    m2q = (e16 == {a8 + b8});
    case (e16) inside [8'(a8 + b8) : 8'(a8 + b8)]: m3 = 1; default: m3 = 0; endcase
    case (e16) inside (a8 + b8): m4 = 1; default: m4 = 0; endcase
    $display("m1=%0d m1q=%0d m2=%0d m2q=%0d m3=%0d m4=%0d", m1, m1q, m2, m2q, m3, m4);
    $finish;
  end
  initial #1000 $finish;
endmodule
