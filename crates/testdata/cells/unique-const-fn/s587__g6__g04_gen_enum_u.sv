module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  if (1) begin : g
    typedef enum int {A = f(2), B} e_t;
    initial begin #1 $display("A=%0d B=%0d", A, B); $finish; end
  end
endmodule
