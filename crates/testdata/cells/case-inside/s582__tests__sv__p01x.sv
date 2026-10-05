`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; casex (v) inside 4'd1: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
