package q;
  function automatic int f(input int a); return 3; endfunction
  int pv = 0;
  task tk(input int x, output int o);
    logic [f(2):0] t;
    t = x + pv;
    o = t;
  endtask
endpackage
module top;
  import q::tk;
  function automatic int f(input int a); return 7; endfunction
  int v;
  initial begin tk(1000, v); $display("v=%0d", v); #1 $finish; end
  initial #50 $finish;
endmodule
