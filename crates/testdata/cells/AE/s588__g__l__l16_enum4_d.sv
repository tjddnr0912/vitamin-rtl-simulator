module top;
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    case (a) 1: t = 4'd5; endcase
    fx1 = t;
  endfunction
  typedef enum logic [3:0] {A = fx1(2)} e_t;
  initial begin #1 $display("A=%b", A); $finish; end
  initial #100 $finish;
endmodule
