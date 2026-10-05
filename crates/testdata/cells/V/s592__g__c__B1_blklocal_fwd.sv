module top;
  localparam K = 4;
  if (1) begin : gb
    initial begin : p
      logic [K-1:0] t;
      t = '1;
      #1 $display("@bits=%0d t=%0d K=%0d", $bits(t), t, K);
    end
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
