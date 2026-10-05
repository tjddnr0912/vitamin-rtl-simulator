module top;
  if (1) begin : g
    localparam integer N = 3;
  end
  for (genvar i = 0; i < g.N; i++) begin : L
    initial $display("@L%0d", i);
  end
  initial #10 $finish;
endmodule
