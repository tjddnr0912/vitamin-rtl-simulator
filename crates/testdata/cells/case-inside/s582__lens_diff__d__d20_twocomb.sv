`timescale 1ns/1ns
module top;
  logic [7:0] a, b, c, d;
  int m1, m2, n1, n2, w;
  logic e1;
  always @* case (a + b) inside [8'd0:8'd9]: m1 = 1; default: m1 = 2; endcase
  always @* case (c + d) inside [8'd0:8'd9]: m2 = 1; default: m2 = 2; endcase
  always @* case (a + c) 8'd5: n1 = 1; default: n1 = 2; endcase
  always @* case (b + d) 8'd5: n2 = 1; default: n2 = 2; endcase
  initial begin
    a = 1; b = 2; c = 4; d = 40;
    #1 $display("t1 m1=%0d m2=%0d n1=%0d n2=%0d", m1, m2, n1, n2);
    a = 20; d = 3;
    #1 $display("t2 m1=%0d m2=%0d n1=%0d n2=%0d", m1, m2, n1, n2);
    e1 = 1'bx;
    case (e1) inside 1'b?: w = 1; default: w = 0; endcase
    $display("w=%0d", w);
    $finish;
  end
  initial #1000 $finish;
endmodule
