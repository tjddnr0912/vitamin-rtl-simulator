module top;
  function automatic logic [3:0] g(input logic [3:0] b);
    g = b;
  endfunction
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    f = g(t);
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
