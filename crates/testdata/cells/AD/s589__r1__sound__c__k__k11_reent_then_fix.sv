package q;
  localparam int W = 3;
  function automatic logic [f(2):0] f(input int a); return a; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
  function automatic logic [W:0] g(); g = '1; endfunction
endpackage
module top;
  function automatic int f(input int a); return 3; endfunction
  localparam int W = 7;
  localparam int P = q::h(1000);
  localparam int Q = q::g();
  initial begin #1 $display("P=%0d Q=%0d", P, Q); $finish; end
  initial #50 $finish;
endmodule
