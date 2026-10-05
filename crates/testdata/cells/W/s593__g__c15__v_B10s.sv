module top;

  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    localparam R = (GS ==? 4'sb1?00);
    initial $display("R=%0d", R);
  end
  initial #100 $finish;
endmodule
