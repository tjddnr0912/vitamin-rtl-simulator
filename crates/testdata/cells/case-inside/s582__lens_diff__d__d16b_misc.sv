`timescale 1ns/1ns
module top;
  typedef struct packed { logic signed [3:0] a; logic [3:0] b; } st_t;
  st_t s; logic signed [3:0] ua [2];
  logic [3:0] v4; logic signed [3:0] s4; logic [7:0] x; int y;
  int m1, m2, m5, m6, w1, w2, w3, w4, o1, o2;
  function automatic int f(input logic [7:0] v);
    case (v + 8'd1) inside [8'd0:8'd9]: return 1; 8'd10: return 2; default: return 0; endcase
  endfunction
  assign y = f(x);
  initial begin
    s = 8'hE5; ua[0] = -4'sd2;
    case (s.a) inside -2: m1 = 1; default: m1 = 0; endcase
    m2 = (s.a == -2);
    case (ua[0]) inside -2: m5 = 1; default: m5 = 0; endcase
    m6 = (ua[0] == -2);
    v4 = 4'b0101; s4 = 4'sb1101;
    case (v4) inside 8'b1???_0101: w1 = 1; 8'b0???_0101: w1 = 2; default: w1 = 0; endcase
    case (s4) inside 8'sb0???_1101: w2 = 1; 8'sb1???_1101: w2 = 2; default: w2 = 0; endcase
    w3 = (v4 inside {8'b1???_0101});
    w4 = (s4 inside {8'sb1???_1101});
    x = 8'd9; #1 o1 = y;
    x = 8'd200; #1 o2 = y;
    $display("m1=%0d m2=%0d m5=%0d m6=%0d w1=%0d w2=%0d w3=%0d w4=%0d o1=%0d o2=%0d", m1, m2, m5, m6, w1, w2, w3, w4, o1, o2);
    $finish;
  end
  initial #1000 $finish;
endmodule
