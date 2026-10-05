module top;
  logic a, b, c, d; int n;
  initial begin
    a = 0; b = 0; c = 0; d = 0; n = 0;
    #1 unique if (a) n = 1; else assert (b) else if (c) n = 2;
    $display("t=%0t A1 n=%0d", $time, n);
    #1 unique if (a) n = 1; else assume (b) else if (c) n = 2;
    $display("t=%0t A2 n=%0d", $time, n);
    #1 unique if (a) n = 1; else assert (b) $display("pass"); else if (c) n = 2;
    $display("t=%0t A3 n=%0d", $time, n);
    #1 unique if (a) n = 1; else if (b) n = 2; else assert (c) else if (d) n = 3;
    $display("t=%0t A4 n=%0d", $time, n);
    #1 priority if (a) n = 1; else assume (b) else if (c) n = 2;
    $display("t=%0t A5 n=%0d", $time, n);
    #1 unique if (a) n = 1; else assert #0 (b) else if (c) n = 2;
    $display("t=%0t A6 n=%0d", $time, n);
    #1 unique if (a) n = 1; else assert final (b) else if (c) n = 2;
    $display("t=%0t A7 n=%0d", $time, n);
    #1 unique if (a) n = 1; else assert (b) else unique if (c) n = 2;
    $display("t=%0t A8 n=%0d", $time, n);
    #1 unique if (a) n = 1; else assert (b) else begin if (c) n = 2; end
    $display("t=%0t A9 n=%0d", $time, n);
    #1 $finish;
  end
endmodule
