module top;
  localparam S = 1;
  if (1) begin : gb
    for (genvar i = 0; i < 3; i = i + S) begin : L
      initial #1 $display("@L%0d", i);
    end
    localparam real S = 1.5;
  end
  initial #5 $finish;
endmodule
