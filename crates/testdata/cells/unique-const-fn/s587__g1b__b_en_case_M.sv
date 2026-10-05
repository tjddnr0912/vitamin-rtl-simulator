module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  typedef enum int {A = f(2), B} e_t;
  initial begin #1 $display("A=%0d", A); $finish; end
endmodule
