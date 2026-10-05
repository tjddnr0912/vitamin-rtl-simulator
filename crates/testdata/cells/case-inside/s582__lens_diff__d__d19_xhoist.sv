`timescale 1ns/1ns
module top;
  logic [3:0] a, b;
  int m1, m2, m3, m4, m5;
  function automatic logic [3:0] fx(input logic [3:0] v); return v; endfunction
  function automatic int g(input logic [3:0] v);
    case (v | 4'b0000) inside 4'b1000: return 1; 4'b1?00: return 2; default: return 0; endcase
  endfunction
  initial begin
    a = 4'b1x00; b = 4'b0000;
    case (a | b) inside 4'b1000: m1 = 1; 4'b1?00: m1 = 2; default: m1 = 0; endcase
    case (fx(a)) inside 4'b1000: m2 = 1; 4'b1?00: m2 = 2; default: m2 = 0; endcase
    case (a + 4'd0) inside 4'b0000: m3 = 1; 4'b????: m3 = 2; default: m3 = 0; endcase
    case (a ^ b) inside [4'd0:4'd15]: m4 = 1; default: m4 = 0; endcase
    m5 = g(a);
    $display("m1=%0d m2=%0d m3=%0d m4=%0d m5=%0d", m1, m2, m3, m4, m5);
    $finish;
  end
  initial #1000 $finish;
endmodule
