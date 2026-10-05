module child (input logic [3:0] p);
  function automatic int cf(input int a);
    cf = 3;
    if (a == 1) cf = 10;
  endfunction
  localparam int L = cf(2);
  logic [L:0] q;
  initial #1 $display("%m L=%0d bq=%0d p=%h", L, $bits(q), p);
endmodule
module top;
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #5 $finish;
endmodule
