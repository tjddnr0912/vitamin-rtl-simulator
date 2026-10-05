`timescale 1ns/1ns
module top;
  logic [63:0] e;
  int m1, m2, m3, m4, m5, m6, m7;
  initial begin
    e = 64'hFFFF_FFFF_0000_0001;
    case (e) inside 'b?1: m1 = 1; default: m1 = 0; endcase
    case (e) inside 'hx: m2 = 1; default: m2 = 0; endcase
    case (e) inside 'bz1: m3 = 1; default: m3 = 0; endcase
    m4 = (e inside {'b?1});
    casez (e) 'b?1: m5 = 1; default: m5 = 0; endcase
    case (e) inside 40'b?1: m6 = 1; default: m6 = 0; endcase
    m7 = (e ==? 'b?1);
    $display("m1=%0d m2=%0d m3=%0d m4=%0d m5=%0d m6=%0d m7=%0d", m1, m2, m3, m4, m5, m6, m7);
    $finish;
  end
  initial #1000 $finish;
endmodule
