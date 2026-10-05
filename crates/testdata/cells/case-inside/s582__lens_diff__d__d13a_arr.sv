`timescale 1ns/1ns
module top;
  int arr [3];
  int v, m1, m2, m3;
  initial begin
    arr = '{1, 5, 9};
    v = 9;
    case (v) inside arr: m1 = 1; default: m1 = 0; endcase
    m2 = (v inside {arr});
    v = 1;
    case (v) inside arr: m3 = 1; default: m3 = 0; endcase
    $display("m1=%0d m2=%0d m3=%0d", m1, m2, m3);
    $finish;
  end
  initial #1000 $finish;
endmodule
