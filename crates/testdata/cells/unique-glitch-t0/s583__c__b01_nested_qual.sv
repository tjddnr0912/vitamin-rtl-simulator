module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1; else unique0 if (b) r = 2;
    $display("t=%0t else-unique0-if done", $time);
    #1 unique if (a) r = 1; else unique if (b) r = 2;
    $display("t=%0t else-unique-if done", $time);
    #1 unique if (a) r = 1; else priority if (b) r = 2;
    $display("t=%0t else-priority-if done", $time);
    #1 unique0 if (a) r = 1; else unique if (b) r = 2;
    $display("t=%0t u0-else-unique-if done", $time);
    #1 unique if (a) r = 1; else L1: if (b) r = 2;
    $display("t=%0t else-labeled-if done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else if (a) r = 3;
    $display("t=%0t chain3-dup done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
