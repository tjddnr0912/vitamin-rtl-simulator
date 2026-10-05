module child #(parameter int W = cf(2)) (input logic [W:0] p);
  function automatic int cf(input int a);
    cf = 3;
    unique if (a == 1) cf = 10;
  endfunction
  initial #1 $display("%m W=%0d b=%0d p=%h", W, $bits(p), p);
endmodule
module top;
  function automatic int cf(input int a);
    cf = 7;
  endfunction
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #5 $finish;
endmodule
