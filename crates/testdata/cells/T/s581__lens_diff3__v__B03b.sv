module top;

  for (genvar i = 8; i < 9; i++) begin : g
    localparam R = (i ==? 4'b1?00);
    initial $display("R=%0d", R);
  end
  initial #100 $finish;
endmodule
