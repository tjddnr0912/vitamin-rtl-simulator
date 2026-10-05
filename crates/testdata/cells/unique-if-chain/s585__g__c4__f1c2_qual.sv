module top;
  logic a, b, c, d; int n;
  initial begin
    a = 0; b = 0; c = 0; d = 0; n = 0;
    #1 unique if (a) n = 1; else (* mark *) if (b) n = 2;
    $display("t=%0t C1 n=%0d", $time, n);
    #1 unique if (a) n = 1; else unique0 if (b) n = 2;
    $display("t=%0t C2 n=%0d", $time, n);
    #1 unique if (a) n = 1; else priority if (b) n = 2;
    $display("t=%0t C3 n=%0d", $time, n);
    #1 unique if (a) begin if (c) n = 3; else if (d) n = 4; end else if (b) n = 2;
    $display("t=%0t C4 n=%0d", $time, n);
    a = 1;
    #1 unique if (a) if (c) n = 3; else if (b) n = 2;
    $display("t=%0t C5 n=%0d", $time, n);
    a = 0;
    #1 unique if (a) if (c) n = 3; else if (b) n = 2;
    $display("t=%0t C6 n=%0d", $time, n);
    #1 unique if (a) n = 1; else unique0 if (b) n = 2; else if (c) n = 3;
    $display("t=%0t C7 n=%0d", $time, n);
    #1 unique if (a) n = 1; else unique if (b) n = 2; else if (c) n = 3;
    $display("t=%0t C8 n=%0d", $time, n);
    #1 unique if (a) n = 1; else if (b) n = 2; else if (c) n = 3; else if (d) n = 4;
    $display("t=%0t C9 n=%0d", $time, n);
    #1 unique0 if (a) n = 1; else if (b) n = 2;
    $display("t=%0t C10 n=%0d", $time, n);
    #1 priority if (a) n = 1; else if (b) n = 2;
    $display("t=%0t C11 n=%0d", $time, n);
    #1 unique if (a) n = 1; else if (b) n = 2; else n = 9;
    $display("t=%0t C12 n=%0d", $time, n);
    #1 unique if (a) n = 1;
       else
         if (b) n = 2;
    $display("t=%0t C13 n=%0d", $time, n);
    #1 $finish;
  end
endmodule
