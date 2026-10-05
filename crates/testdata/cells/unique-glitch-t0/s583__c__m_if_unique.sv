module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1;
    $display("t=%0t if1 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t chain2 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else if (c) r = 3;
    $display("t=%0t chain3 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else r = 3;
    $display("t=%0t chain-else done", $time);
    #1 unique if (a) r = 1; else begin if (b) r = 2; end
    $display("t=%0t else-begin-if done", $time);
    #1 unique if (a) begin if (c) r = 1; end else if (b) r = 2;
    $display("t=%0t nested-in-then done", $time);
    #1 unique if (a) if (c) r = 1; else r = 2;
    $display("t=%0t dangling-else done", $time);
    #1 a = 1; unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t chain-match done", $time);
    #1 a = 0; b = 1; unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t chain-match2 done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
