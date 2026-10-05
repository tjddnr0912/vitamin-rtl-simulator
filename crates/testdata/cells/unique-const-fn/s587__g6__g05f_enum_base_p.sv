module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  typedef enum logic [f(2):0] {A = 8'hff, B = 0} e_t;
  e_t e = A;
  initial begin #1 $display("b=%0d e=%h", $bits(e), e); $finish; end
endmodule
