module top;
  localparam N = 1;
  if (1) begin : gb
    for (genvar i = 0; i < N; i++) begin : L
      wire [7:0] w = 8'd10 + i;
      initial #1 $display("@L%0d w=%0d", i, w);
    end
    localparam N = 3;
  end
  initial #5 $finish;
endmodule
