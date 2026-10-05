module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  if (fx(2) == 4'd0) begin : g1 initial $display("T"); end else begin : g2 initial $display("E"); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
