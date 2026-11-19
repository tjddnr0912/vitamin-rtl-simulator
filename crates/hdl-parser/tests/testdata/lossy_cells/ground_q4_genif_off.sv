module t;
  if (0) begin : g
    localparam unsigned U = 4'sd8;
    initial $display("G u=%0d", U);
  end
  localparam V = 4'd8;
  initial begin #1 $display("A v=%0d gt=%b", V, V > 4'sd0); end
  initial #100 $finish;
endmodule
