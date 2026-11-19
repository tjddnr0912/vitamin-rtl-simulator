module t #(parameter logic [3:0] P = 4'd3)();
  if (P == 'x) begin : g1 initial #1 $display("A g1"); end else begin : g2 initial #1 $display("A g2"); end
  initial #100 $finish;
endmodule
