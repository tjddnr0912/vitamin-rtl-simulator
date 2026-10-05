module top;
  logic a, b, c; int n; event e;
  initial #12 -> e;
  initial begin
    a = 0; b = 0; c = 0; n = 0;
    #1 unique if (a) n = 1; else begin if (b) n = 2; end
    $display("t=%0t B1 n=%0d", $time, n);
    #1 unique if (a) n = 1; else L2: if (b) n = 2;
    $display("t=%0t B2 n=%0d", $time, n);
    #1 unique if (a) n = 1; else ;
    $display("t=%0t B3 n=%0d", $time, n);
    #1 unique if (a) n = 1; else #1 if (b) n = 2;
    $display("t=%0t B4 n=%0d", $time, n);
    #1 unique if (a) n = 1; else case (b) 1'b1: n = 2; endcase
    $display("t=%0t B5 n=%0d", $time, n);
    #1 unique if (a) n = 1; else wait (1) if (b) n = 2;
    $display("t=%0t B6 n=%0d", $time, n);
    #1 unique if (a) n = 1; else if (b) n = 2; else begin end
    $display("t=%0t B7 n=%0d", $time, n);
    #1 unique if (a) n = 1; else @(e) if (b) n = 2;
    $display("t=%0t B8 n=%0d", $time, n);
    #1 unique if (a) n = 1; else repeat (1) if (b) n = 2;
    $display("t=%0t B9 n=%0d", $time, n);
    #1 $finish;
  end
endmodule
