`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [3:0] v; int m;
  initial begin
    v = -4'sd1; case (v) inside [4'd1:4'd3]: m = 1; [-1:1]: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
