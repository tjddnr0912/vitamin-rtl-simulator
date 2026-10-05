`timescale 1ns/1ns
module top;
  logic [63:0] e;
  int m1, m2, m3, m4;
  initial begin
    e = 64'hFFFF_FFFF_0000_0001;
    casez (e) 'b?1: m1 = 1; default: m1 = 0; endcase
    casex (e) 'bx1: m2 = 1; default: m2 = 0; endcase
    m3 = (e inside {'b?1});
    m4 = (e ==? 'b?1);
    $display("casez m1=%0d casex m2=%0d inside m3=%0d wildeq m4=%0d", m1, m2, m3, m4);
    $finish;
  end
  initial #1000 $finish;
endmodule
