module sub(input logic [3:0] p);
  initial #1 if (p == 4'h7) $display("hit %m");
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [31:0] w = 32'h76543210;
  sub u [f(2):0] (.p(w));
  initial #2 $finish;
endmodule
