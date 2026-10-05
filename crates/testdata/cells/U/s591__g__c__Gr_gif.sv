module top;
  localparam real i = 2.5;
  for (genvar i = 0; i < 2; i++) begin : g
    if (i == 1) begin : t initial #1 $display("gif %m hit"); end
    else begin : e initial #1 $display("gif %m miss"); end
  end
  initial #5 $display("post %f", i);
  initial #100 $finish;
endmodule
