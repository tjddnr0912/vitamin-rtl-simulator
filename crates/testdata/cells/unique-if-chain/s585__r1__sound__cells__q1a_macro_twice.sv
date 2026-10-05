`define CHAIN(A,B,C) unique if (A) r = 1; else if (B) r = 2; else if (C) r = 3;
module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  initial begin
    #1 `CHAIN(a,b,c)
    $display("t=%0t m1", $time);
    #1 `CHAIN(a,b,c)
    $display("t=%0t m2", $time);
    #1 c = 1; `CHAIN(a,b,c)
    $display("t=%0t m3 r=%0d", $time, r);
    #1 $finish;
  end
endmodule
