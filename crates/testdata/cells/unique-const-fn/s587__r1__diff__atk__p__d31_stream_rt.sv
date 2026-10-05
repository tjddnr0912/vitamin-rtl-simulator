module top;
  function automatic int f(input int a);
    f = 4;
    if (a == 1) f = 10;
  endfunction
  logic [15:0] x = 16'h1234, y;
  initial begin y = {<< f(2) {x}}; #1 $display("y=%h", y); $finish; end
endmodule
