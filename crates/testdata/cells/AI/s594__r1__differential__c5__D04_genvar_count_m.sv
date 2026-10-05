module t;
  for (genvar g = 4; g <= 16; g += 12) begin : G
    localparam L = ((({g{1'b0}} + 8'd255 + 8'd1) >> 1) == 8'd128);
    initial $display("R: g=%0d L=%0d", g, L);
  end
  initial #10 $finish;
endmodule
