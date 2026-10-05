`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd15; case (v) inside 1, 3: m = 1; [4:7]: m = 2; '1: m = 3; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
