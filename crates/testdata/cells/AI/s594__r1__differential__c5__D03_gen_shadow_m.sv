module t;
  localparam N = 4;
  if (1) begin : G
    localparam N = 16;
    localparam L = ((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 8'd128);
    initial $display("R: L=%0d", L);
  end
  initial #10 $finish;
endmodule
