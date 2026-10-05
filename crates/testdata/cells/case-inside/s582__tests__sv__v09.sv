`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] v; logic signed [7:0] s; int m;
  task automatic tu(input logic [7:0] x);
    v = x;
    case (v) inside (-4'sd8 + 8'sd0): m = 1; default: m = 0; endcase
    $display("u v=%h m=%0d", v, m);
  endtask
  task automatic ts(input logic signed [7:0] x);
    s = x;
    case (s) inside (-4'sd8 + 8'sd0): m = 1; default: m = 0; endcase
    $display("s s=%h m=%0d", s, m);
  endtask
  initial begin
    tu(8'hF8); tu(8'h08); ts(8'shF8); ts(8'sh08);
    $finish;
  end
endmodule
