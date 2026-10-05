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
  for (genvar i = 0; i < 2; i++) begin : g
    child c (.p(bus[i*4 +: 4]));
  end
  initial #5 $finish;
endmodule
