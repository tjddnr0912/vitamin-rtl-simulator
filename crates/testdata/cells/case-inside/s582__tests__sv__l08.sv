`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  real r; int m;
  initial begin
    r = 2.5; case (r) inside 1.0: m = 1; [2.0:3.0]: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
