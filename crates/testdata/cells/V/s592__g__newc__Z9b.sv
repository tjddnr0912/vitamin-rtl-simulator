module top;
  localparam S = 0;
  if (1) begin : gb
    for (genvar i = S; i < 2; i++) begin : L
      initial #1 $display("@L%0d", i);
    end
    localparam S = Nope;
  end
  initial #5 $finish;
endmodule
