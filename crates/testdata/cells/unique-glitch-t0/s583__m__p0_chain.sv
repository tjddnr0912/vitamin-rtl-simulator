module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  initial begin
    #1 priority0 if (a) r = 1; else if (b) r = 2;
    $display("t=%0t p0 chain2 done", $time);
    #1 priority0 if (a) r = 1; else if (b) r = 2; else if (c) r = 3;
    $display("t=%0t p0 chain3 done", $time);
    #1 unique if (a) r = 1; else priority0 if (b) r = 2;
    $display("t=%0t else-priority0-if done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
