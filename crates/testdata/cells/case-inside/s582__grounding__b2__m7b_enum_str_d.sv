module top;
  typedef enum logic [2:0] {R_F, I_F, S_F, B_F, U_F, J_F} fmt_t;
  string s; fmt_t f; int m;
  initial begin
    s = "ab";
    case (1'b1) (s inside {"aa", "ab"}): m = 1; default: m = 0; endcase $display("s m=%0d", m);
    s = "zz";
    case (1'b1) (s inside {"aa", "ab"}): m = 1; default: m = 0; endcase $display("s2 m=%0d", m);
    for (int i = 0; i < 6; i++) begin
      f = fmt_t'(i);
      case (1'b1) (f inside {R_F}): m = 1; (f inside {S_F, B_F}): m = 2; (f inside {[U_F:J_F]}): m = 3; default: m = 0; endcase
      $display("f=%s m=%0d", f.name(), m);
    end
    #10 $finish;
  end
endmodule
