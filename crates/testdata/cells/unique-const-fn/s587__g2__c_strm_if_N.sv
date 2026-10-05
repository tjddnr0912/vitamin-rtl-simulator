module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [15:0] x = 16'h1234;
  logic [15:0] y;
  initial begin #1 y = {<< f(1) {x}}; $display("st=%h", y); $finish; end
endmodule
