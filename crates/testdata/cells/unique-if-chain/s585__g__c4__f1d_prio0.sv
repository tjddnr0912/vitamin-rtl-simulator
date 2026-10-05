module top;
  logic a, b, c; int n;
  initial begin
    a = 0; b = 0; c = 0; n = 0;
    #1 unique if (a) n = 1; else priority0 if (b) n = 2;
    $display("t=%0t D1 n=%0d", $time, n);
    #1 priority if (a) n = 1; else priority0 if (b) n = 2; else if (c) n = 3;
    $display("t=%0t D2 n=%0d", $time, n);
    #1 $finish;
  end
endmodule
