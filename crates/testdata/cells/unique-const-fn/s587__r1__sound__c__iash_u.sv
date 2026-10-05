package q;
  function automatic int g(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
endpackage
module child #(parameter int W = q::g(2)) (input logic [W-1:0] p);
  initial #1 $display("%m W=%0d p=%b", W, p);
endmodule
module top;
  localparam int W = 8;
  logic [13:0] bus = 14'b10101011001100;
  child u[1:0] (.p(bus));
  initial #5 $finish;
endmodule
