`timescale 1ns/1ns
module top;
  int q [$];
  int da [];
  int v, m1, m2, m3, m4;
  initial begin
    q = {4, 6}; da = new[2]; da[0] = 11; da[1] = 13;
    v = 6;
    case (v) inside q: m1 = 1; default: m1 = 0; endcase
    m2 = (v inside {q});
    v = 13;
    case (v) inside da: m3 = 1; default: m3 = 0; endcase
    m4 = (v inside {da});
    $display("m1=%0d m2=%0d m3=%0d m4=%0d", m1, m2, m3, m4);
    $finish;
  end
  initial #1000 $finish;
endmodule
