`timescale 1ns/1ns
module top;
  typedef enum logic signed [3:0] {NEG = -2, POS = 3} se_t;
  typedef struct packed signed { logic [3:0] hi; logic [3:0] lo; } ps_t;
  logic [7:0] v8; logic signed [15:0] s16; ps_t ps; logic signed [7:0] s8;
  int m1, m2, m3, m4, m5, m6;
  initial begin
    v8 = 8'hFE;
    case ($signed(v8)) inside -2: m1 = 1; default: m1 = 0; endcase
    s16 = -16'sd1;
    case (s16) inside 8'(4'sb1111): m2 = 1; default: m2 = 0; endcase
    case (s16) inside signed'(8'hFF): m3 = 1; default: m3 = 0; endcase
    s8 = -8'sd2;
    case (s8) inside NEG: m4 = 1; POS: m4 = 2; default: m4 = 0; endcase
    ps = 8'hFE;
    case (ps) inside -16'sd2: m5 = 1; default: m5 = 0; endcase
    case (ps) inside [-16'sd3 : -16'sd1]: m6 = 1; default: m6 = 0; endcase
    $display("m1=%0d m2=%0d m3=%0d m4=%0d m5=%0d m6=%0d", m1, m2, m3, m4, m5, m6);
    $finish;
  end
  initial #1000 $finish;
endmodule
