`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  class C; int x; endclass
  C h; int m;
  initial begin
    h = null;
    case (h) inside null: m = 1; default: m = 0; endcase
    $display("m=%0d", m);
    $finish;
  end
endmodule
