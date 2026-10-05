module top;
  localparam N = 1;
  if (1) begin : gb
    for (genvar i = 0; i < N; i++) begin : L
      initial #1 $display("@L%0d", i);
    end
    localparam N = 3;
  end
  initial #5 $finish;
endmodule
