`timescale 1ns/1ns
module top;
  logic signed [7:0] s;
  int m1, m2, m3, m4;
  initial begin
    s = 8'sb1010_0011;
    case (s) inside 4'sbx011: m1 = 1; default: m1 = 0; endcase
    case (s) inside 4'sb?011: m2 = 1; default: m2 = 0; endcase
    m3 = (s inside {4'sbx011});
    s = 8'sb0000_0011;
    case (s) inside 4'sb1011: m4 = 1; default: m4 = 0; endcase
    $display("m1=%0d m2=%0d m3=%0d m4=%0d", m1, m2, m3, m4);
    $finish;
  end
  initial #1000 $finish;
endmodule
