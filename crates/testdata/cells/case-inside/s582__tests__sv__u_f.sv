`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; default: m = 0; endcase $display("v=%b m=%0d", v, m);
    v = 4'd12; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; default: m = 0; endcase $display("v=%b m=%0d", v, m);
    v = 4'd3; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; default: m = 0; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
