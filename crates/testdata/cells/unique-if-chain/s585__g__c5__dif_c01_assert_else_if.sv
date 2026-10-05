module top;
  logic a, b, c; logic [1:0] y;
  initial begin
    a = 0; b = 0; c = 0; y = 0;
    #1 unique if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t u-assert-fail-elseif y=%0d", $time, y);
    #1 b = 1; unique if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t u-assert-pass y=%0d", $time, y);
    #1 b = 0; unique if (a) y = 1; else assume (b) else if (c) y = 2;
    $display("t=%0t u-assume-fail-elseif y=%0d", $time, y);
    #1 unique if (a) y = 1; else assert (b) $display("  pass-action"); else if (c) y = 2;
    $display("t=%0t u-assert-passact-fail-elseif y=%0d", $time, y);
    #1 priority if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t p-assert-fail-elseif y=%0d", $time, y);
    #1 c = 1; unique if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t u-assert-fail-elseif-c1 y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
