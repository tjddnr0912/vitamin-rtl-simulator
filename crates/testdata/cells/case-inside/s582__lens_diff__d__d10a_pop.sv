`timescale 1ns/1ns
module top;
  int q[$];
  int m1, m2;
  initial begin
    q = '{3, 2, 1, 5, 6, 7};
    case (q.pop_front()) inside 1: m1 = 1; 2: m1 = 2; 3: m1 = 3; default: m1 = 0; endcase
    $display("ci pop m1=%0d size=%0d", m1, q.size());
    case (q.pop_front()) 1: m2 = 1; 2: m2 = 2; 3: m2 = 3; default: m2 = 0; endcase
    $display("plain pop m2=%0d size=%0d", m2, q.size());
    $finish;
  end
  initial #1000 $finish;
endmodule
