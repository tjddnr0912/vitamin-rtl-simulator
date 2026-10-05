`timescale 1ns/1ns
module top;
  logic [3:0] a4, b4;
  int r1, r2, r3, r4, r5, m;
  initial begin
    a4 = 4'hF; b4 = 4'h1;
    r1 = ((a4 + b4) inside {4'h0, 8'hFF});
    r2 = ((a4 + b4) inside {8'h10});
    r3 = ((a4 + b4) inside {4'h0});
    r4 = ((a4 + b4) inside {[4'h0:4'h0], 8'hFF});
    r5 = ((a4 + b4) inside {8'hFF, 4'h0});
    $display("q6 r1=%0d r2=%0d r3=%0d r4=%0d r5=%0d", r1, r2, r3, r4, r5);
    $finish;
  end
  initial #1000 $finish;
endmodule
