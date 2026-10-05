`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  string s; int m;
  initial begin
    s = "ab";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    s = "zz";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    $finish;
  end
endmodule
