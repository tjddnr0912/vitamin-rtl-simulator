`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] v8; logic [3:0] a4, b4; int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10; case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
