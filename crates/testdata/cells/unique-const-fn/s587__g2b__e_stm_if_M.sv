module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  typedef struct packed { logic [f(2):0] a; logic b; } s_t;
  s_t s;
  initial begin #1 $display("b=%0d", $bits(s)); $finish; end
endmodule
