`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  import p::*;
  logic [3:0] x; int m;
  initial begin
    x = 4'd5;
    case (x) inside [4'd4:4'd6]: m = 1; default: m = 0; endcase
    $display("x m=%0d", m);
    $finish;
  end
endmodule
