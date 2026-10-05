module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  if (fd(2) == 2) begin : g1 initial $display("T"); end else begin : g2 initial $display("E"); end
  initial begin #2 $display("done"); $finish; end
  initial #100 $finish;
endmodule
