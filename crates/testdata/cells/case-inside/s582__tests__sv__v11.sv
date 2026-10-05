`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  typedef enum logic [2:0] {R_F, I_F, S_F, B_F, U_F, J_F} fmt_t;
  string s; fmt_t f; int m;
  initial begin
    s = "ab";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    s = "zz";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    for (int i = 0; i < 6; i++) begin
      f = fmt_t'(i);
      case (f) inside R_F: m = 1; S_F, B_F: m = 2; [U_F:J_F]: m = 3; default: m = 0; endcase
      $display("f=%0d m=%0d", f, m);
    end
    $finish;
  end
endmodule
