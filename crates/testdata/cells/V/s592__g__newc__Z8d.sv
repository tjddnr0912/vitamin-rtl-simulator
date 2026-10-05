module top;
  localparam S = 1;
  if (1) begin : gb
    for (genvar i = 0; i < 3; i = i + S) begin : L
      initial #1 $display("@L%0d", i);
    end
    localparam logic [71:0] S = 72'hFF_0000_0000_0000_0001;
  end
  initial #5 $finish;
endmodule
