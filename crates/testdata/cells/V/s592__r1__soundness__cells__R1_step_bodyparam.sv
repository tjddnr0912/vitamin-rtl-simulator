module top;
  localparam S = 2;
  for (genvar i = 0; i < 4; i = i + S) begin : g
    localparam S = 1;
    initial #1 $display("@%m");
  end
  initial #5 $finish;
endmodule
