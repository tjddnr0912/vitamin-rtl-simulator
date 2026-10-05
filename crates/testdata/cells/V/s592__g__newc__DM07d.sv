module top;
  localparam [63:0] S = 64'h0000_0100_0000_0000;
  if (1) begin : g
    for (genvar i = 0; i < S[41:40] + 2; i = i + 1) begin : L
      initial $display("@L%0d", i);
    end
    localparam integer S = 3;
  end
  initial #10 $finish;
endmodule
