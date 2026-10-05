module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  typedef enum logic [3:0] {A = fd(2), B} e_t;
  initial begin #2 $display("A=%0d B=%0d", A, B); $finish; end
  initial #100 $finish;
endmodule
