module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  typedef struct packed { logic [f(2):0] a; logic b; } s_t;
  s_t s;
  initial begin #1 $display("b=%0d", $bits(s)); $finish; end
endmodule
