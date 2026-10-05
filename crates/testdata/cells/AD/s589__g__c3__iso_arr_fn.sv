module child (input logic [3:0] p);
  localparam int L = f(2);
  initial #1 $display("%m L=%0d", L);
endmodule
module top;
  function automatic int f(input int a); return 7; endfunction
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
