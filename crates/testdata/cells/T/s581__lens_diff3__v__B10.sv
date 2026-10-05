module top;

  if (1) begin : gb
    localparam logic [39:0] GP = 40'h10_0000_000C;
    localparam R = (GP ==? 4'b1?00);
    initial $display("R=%0d", R);
  end
  initial #100 $finish;
endmodule
