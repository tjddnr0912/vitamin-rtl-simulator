module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  typedef enum int {A = f(2), B} e_t;
  e_t e;
  initial begin e = B; #1 $display("e=%0d n=%s A=%0d", e, e.name(), A); $finish; end
endmodule
