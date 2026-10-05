module sub(input logic [f(2)-4:0] p);
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  initial #1 if (p == 4'h7) $display("hit %m b=%0d", $bits(p));
endmodule
module top;
  logic [31:0] w = 32'h76543210;
  sub u [7:0] (.p(w));
  initial #2 $finish;
endmodule
