module sub #(parameter int W = f(2)) (input logic [W-4:0] p);
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  initial #1 if (p == 4'h7) $display("hit %m b=%0d W=%0d", $bits(p), W);
endmodule
module top;
  logic [31:0] w = 32'h76543210;
  sub u [7:0] (.p(w));
  initial #2 $finish;
endmodule
