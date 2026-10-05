`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int q[$]; int x; int m;
  initial begin
    q = '{1, 5, 9};
    x = 9; case (x) inside q[$]: m = 1; default: m = 0; endcase $display("x=%0d m=%0d", x, m);
    $finish;
  end
endmodule
