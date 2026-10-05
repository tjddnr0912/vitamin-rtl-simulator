module top;
  localparam S = 0;
  if (1) begin : gb
    for (genvar i = S; i < 2; i++) begin : L
      initial #1 $display("@L%0d", i);
    end
    localparam logic [71:0] S = 72'hFF_0000_0000_0000_0001;
  end
  initial #5 $finish;
endmodule
