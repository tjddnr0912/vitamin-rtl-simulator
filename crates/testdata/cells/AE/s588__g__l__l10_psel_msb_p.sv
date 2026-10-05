module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  localparam logic [7:0] V = 8'hA5;
  initial begin #1 $display("r=%b", V[fx(2):0]); $finish; end
  initial #100 $finish;
endmodule
