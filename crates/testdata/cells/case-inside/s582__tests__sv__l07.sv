`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'b1000; case (v) inside {2'b1?, 2'b00}: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
