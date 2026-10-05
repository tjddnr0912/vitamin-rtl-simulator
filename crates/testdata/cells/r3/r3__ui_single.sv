`timescale 1ns/1ns
module t;
  logic a, b; logic [1:0] r;
  initial begin
    a = 0; b = 0; r = 2'd0;
    #1;
    unique if (a) r = 2'd1;
    $display("t=%0t r=%0d", $time, r);
    #1 $finish;
  end
endmodule
