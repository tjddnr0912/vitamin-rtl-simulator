module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      initial $display("@same");
    end else begin : b
      initial $display("@same");
    end
    localparam integer K = 2;
  end
  initial #10 $finish;
endmodule
