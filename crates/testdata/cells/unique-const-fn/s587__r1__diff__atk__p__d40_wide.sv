module top;
  function automatic int fw(input int a);
    fw = 99;
    if (a == 1) fw = 10;
  endfunction
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  parameter [fw(2):0] W = '1;
  localparam logic [f(2):0] Q = 9'h1fe;
  parameter logic signed [f(2):0] S = -2;
  initial begin #1 $display("bW=%0d W99=%b bQ=%0d Q=%h bS=%0d S=%0d", $bits(W), W[99], $bits(Q), Q, $bits(S), S); $finish; end
endmodule
