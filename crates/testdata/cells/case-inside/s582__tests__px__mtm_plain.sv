`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] a, b; int m;
  initial begin
    a = 8'h80; b = 8'h80;
    case ((1:a + b:2)) 9'h100: m = 1; default: m = 0; endcase $display("mtm m=%0d", m);
    case ((a + b)) 9'h100: m = 1; default: m = 0; endcase $display("paren m=%0d", m);
    $finish;
  end
endmodule
