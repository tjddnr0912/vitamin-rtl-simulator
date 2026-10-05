module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  function automatic int g(input int a);
    logic [f(2):0] t;
    t = '1;
    return $bits(t) + a;
  endfunction
  int n = 0;
  initial begin repeat (g(0)) n++; #(g(0)) $display("n=%0d t=%0t", n, $time); $finish; end
endmodule
