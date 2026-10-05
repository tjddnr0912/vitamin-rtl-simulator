`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] a, b; int m;
  initial begin
    a = 8'h80; b = 8'h80; case (a + b) inside 8'h00: m = 1; 16'h0100: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
