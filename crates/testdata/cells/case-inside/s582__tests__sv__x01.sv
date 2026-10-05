`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [15:0] v; int m;
  initial begin
    v = 16'h6162; case (v) inside "ab": m = 1; default: m = 0; endcase $display("m=%0d", m);
    v = 16'h6163; case (v) inside "ab": m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
