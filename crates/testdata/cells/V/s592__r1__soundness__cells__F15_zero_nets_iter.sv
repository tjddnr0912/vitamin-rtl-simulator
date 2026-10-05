module top;
  localparam N = 0;
  if (1) begin : b
    for (genvar i = 0; i < N; i = i + 1) begin : g
      initial #1 $display("@%m");
    end
    localparam N = 2;
  end
  initial #5 $finish;
endmodule
