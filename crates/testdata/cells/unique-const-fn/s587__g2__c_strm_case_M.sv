module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic [15:0] x = 16'h1234;
  logic [15:0] y;
  initial begin #1 y = {<< f(2) {x}}; $display("st=%h", y); $finish; end
endmodule
