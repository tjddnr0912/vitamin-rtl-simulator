package pk;
  function void fv(input int x);
    int r; r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
  endfunction
  task t(input int x);
    int r; r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
  endtask
endpackage
module top;
  import pk::*;
  initial begin #1 fv(0); $display("t=%0t", $time); #2 t(0); $display("t=%0t", $time); #1 $finish; end
endmodule
