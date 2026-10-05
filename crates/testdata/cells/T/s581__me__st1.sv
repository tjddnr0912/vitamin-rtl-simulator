module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam [64:0] Q = {64'd0, (i ==? 32'b???1)};
    localparam int K = i;
    initial $display("i=%0d Q=%0d K=%0d", i, Q, K);
  end
endmodule
