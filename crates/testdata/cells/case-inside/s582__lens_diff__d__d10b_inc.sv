`timescale 1ns/1ns
module top;
  int cnt;
  int m3, m4;
  initial begin
    cnt = 5;
    case (cnt++) inside 1: m3 = 1; 2: m3 = 2; 5: m3 = 5; default: m3 = 0; endcase
    $display("ci inc m3=%0d cnt=%0d", m3, cnt);
    cnt = 5;
    case (cnt++) 1: m4 = 1; 2: m4 = 2; 5: m4 = 5; default: m4 = 0; endcase
    $display("plain inc m4=%0d cnt=%0d", m4, cnt);
    $finish;
  end
  initial #1000 $finish;
endmodule
