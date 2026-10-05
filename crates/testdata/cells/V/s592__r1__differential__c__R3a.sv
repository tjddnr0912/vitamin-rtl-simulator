module top;
  localparam integer N = 1;
  if (1) begin : g
    for (genvar i = 0; i < N; i++) begin : L
      if (i == 0) begin : p
        initial $display("@L%0d", i);
      end
    end
    localparam integer N = 3;
  end
  initial #10 $finish;
endmodule
