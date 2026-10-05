module top;
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    if (a == 1) t = 4'd5;
    fx1 = t;
  endfunction
  localparam int P = fx1(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
