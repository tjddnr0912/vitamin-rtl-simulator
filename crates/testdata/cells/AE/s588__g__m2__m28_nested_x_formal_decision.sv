module top;
  function automatic logic [3:0] g(input logic [3:0] b);
    if (b == 4'd0) g = 4'd1; else g = 4'd2;
  endfunction
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    f = g(t);
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #2 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
