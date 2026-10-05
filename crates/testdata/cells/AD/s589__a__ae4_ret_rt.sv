package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    logic [3:0] t;
    h = t;
  endfunction
endpackage
module top;
  logic [3:0] v;
  initial begin v = q::h(2); $display("v=%b", v); #1 $finish; end
  initial #50 $finish;
endmodule
