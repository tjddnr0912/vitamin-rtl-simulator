module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  typedef enum int {A = f(1), B} e_t;
  initial begin #1 $display("A=%0d", A); $finish; end
endmodule
