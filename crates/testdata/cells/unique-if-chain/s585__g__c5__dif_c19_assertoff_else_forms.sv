module top;
  logic a, b; logic [1:0] y; logic [3:0] s; int i;
  initial begin
    a = 0; b = 0; y = 0; s = 4'd9;
    #1 $assertoff; unique if (a) y = 1; else if (b) y = 2;
    $display("t=%0t chain-under-assertoff", $time);
    #1 unique if (a) y = 1;
    $display("t=%0t lone-under-assertoff", $time);
    #1 $asserton; unique if (a) y = 1; else case (s) inside [0:3]: y = 3; endcase
    $display("t=%0t else-case-inside y=%0d", $time, y);
    #1 unique if (a) y = 1; else do i++; while (i < 2);
    $display("t=%0t else-do-while i=%0d", $time, i);
    #1 unique if (a) y = 1; else if (b) y = 2; else for (i = 0; i < 1; i++) ;
    $display("t=%0t else-for", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
