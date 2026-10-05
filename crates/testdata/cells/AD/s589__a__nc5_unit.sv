package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
localparam int U = q::h(18);
module top;
  initial begin #1 $display("U=%0d", U); $finish; end
  initial #50 $finish;
endmodule
