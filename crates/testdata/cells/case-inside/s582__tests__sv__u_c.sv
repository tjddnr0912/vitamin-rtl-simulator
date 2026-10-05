`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9; unique0 case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    v = 4'd5; m = 9; unique0 case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
