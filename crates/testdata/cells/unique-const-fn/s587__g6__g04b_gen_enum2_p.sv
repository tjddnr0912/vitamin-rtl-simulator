module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  if (1) begin : g
    typedef enum int {A = f(2), B} e_t;
    e_t e = B;
    initial begin #1 $display("e=%0d", e); $finish; end
  end
endmodule
