module top;
  int n; logic [1:0] y;
  function automatic bit f(input int k, input bit v); n++; $display("  eval c%0d", k); return v; endfunction
  initial begin
    n = 0;
    #1 if (f(1,0)) y = 1; else if (f(2,0)) y = 2; else unique if (f(3,0)) y = 3;
    $display("t=%0t miss n=%0d", $time, n);
    #1 n = 0; if (f(1,0)) y = 1; else if (f(2,1)) y = 2; else unique if (f(3,0)) y = 3;
    $display("t=%0t hit2 n=%0d y=%0d", $time, n, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
