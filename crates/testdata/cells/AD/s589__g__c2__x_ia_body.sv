module child (input logic [3:0] p);
  function automatic int f(input int a); return 3; endfunction
  localparam int L = f(2);
  logic [f(2):0] q;
  initial #1 $display("%m L=%0d bq=%0d", L, $bits(q));
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
