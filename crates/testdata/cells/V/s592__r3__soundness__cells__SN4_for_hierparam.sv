module top;
  localparam N = 1;
  if (1) begin : gb
    for (genvar i = 0; i < N; i++) begin : L localparam P = i + 10; end
    localparam N = 3;
  end
  initial #1 $display("@P=%0d", gb.L[2].P);
  initial #5 $finish;
endmodule
