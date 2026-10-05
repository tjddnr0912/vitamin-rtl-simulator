module top;
  localparam logic [1:0][69:0] PP70 = {70'hC, 70'h0};
  localparam R = (PP70[1] ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
