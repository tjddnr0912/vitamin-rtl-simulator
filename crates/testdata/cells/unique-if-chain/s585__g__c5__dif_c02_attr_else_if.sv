module top;
  logic a, b; logic [1:0] y;
  initial begin
    a = 0; b = 0; y = 0;
    #1 unique if (a) y = 1; else (* mark *) if (b) y = 2;
    $display("t=%0t attr-elseif done", $time);
    #1 unique (* mark *) if (a) y = 1; else if (b) y = 2;
    $display("t=%0t attr-after-unique done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
