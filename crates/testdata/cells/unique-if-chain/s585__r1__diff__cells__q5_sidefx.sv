module top;
  int n1 = 0, n2 = 0, n3 = 0, i = 0;
  logic [1:0] r = 0;
  function automatic bit f1(input bit v); n1++; return v; endfunction
  function automatic bit f2(input bit v); n2++; return v; endfunction
  function automatic bit f3(input bit v); n3++; return v; endfunction
  initial begin
    #1 unique if (f1(0)) r = 1; else if (f2(0)) r = 2; else if (f3(0)) r = 3;
    $display("t=%0t miss n=%0d %0d %0d", $time, n1, n2, n3);
    #1 unique if (f1(0)) r = 1; else if (f2(1)) r = 2; else if (f3(0)) r = 3;
    $display("t=%0t hit2 n=%0d %0d %0d", $time, n1, n2, n3);
    #1 priority if (f1(0)) r = 1; else if (f2(0)) r = 2;
    $display("t=%0t prio miss n=%0d %0d %0d", $time, n1, n2, n3);
    #1 unique if (f1(i == 5)) r = 1; else if (f2(i == 7)) r = 2;
    $display("t=%0t incr i=%0d", $time, i);
    #1 unique if (f1(0)) r = 1; else if (f2(0)) r = 2; else r = 0;
    $display("t=%0t else n=%0d %0d %0d", $time, n1, n2, n3);
    #1 $finish;
  end
endmodule
