`timescale 1ns/1ns
module t;
  logic a, b; logic [1:0] r;
  initial begin
    a = 1; b = 1; r = 2'd0;
    #1;
    priority if (a) r = 2'd1;
    else if (b) r = 2'd2;
    $display("t=%0t r=%0d", $time, r);
    #1 $finish;
  end
endmodule
