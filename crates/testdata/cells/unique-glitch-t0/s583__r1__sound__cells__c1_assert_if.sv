module top;
  logic a = 0, b = 0, c = 0; integer x = 0;
  initial begin
    #1 unique if (a) x = 1; else assert (b) else if (c) $display("t=%0t c-branch", $time);
    $display("t=%0t A1 done", $time);
    #1 c = 1;
    unique if (a) x = 1; else assert (b) else if (c) $display("t=%0t c-branch", $time);
    #1 b = 1; c = 0;
    unique if (a) x = 1; else assert (b) else if (c) $display("t=%0t c-branch", $time);
    $display("t=%0t A3 done", $time);
    #1 b = 0;
    priority if (a) x = 1; else if (c) x = 2; else assume (b) else if (c) $display("t=%0t never", $time);
    $display("t=%0t A4 done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
