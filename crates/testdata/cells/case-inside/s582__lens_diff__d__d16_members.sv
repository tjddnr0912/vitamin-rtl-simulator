`timescale 1ns/1ns
module top;
  typedef struct packed { logic signed [3:0] a; logic [3:0] b; } st_t;
  typedef logic signed [3:0] s4_t;
  st_t s; s4_t [1:0] arr2; logic signed [3:0] ua [2];
  int m1, m2, m3, m4, m5, m6;
  initial begin
    s = 8'hE5; arr2 = 8'hE0; ua[0] = -4'sd2;
    case (s.a) inside -2: m1 = 1; default: m1 = 0; endcase
    m2 = (s.a == -2);
    case (arr2[1]) inside -2: m3 = 1; default: m3 = 0; endcase
    m4 = (arr2[1] == -2);
    case (ua[0]) inside -2: m5 = 1; default: m5 = 0; endcase
    m6 = (ua[0] == -2);
    $display("m1=%0d m2=%0d m3=%0d m4=%0d m5=%0d m6=%0d", m1, m2, m3, m4, m5, m6);
    $finish;
  end
  initial #1000 $finish;
endmodule
