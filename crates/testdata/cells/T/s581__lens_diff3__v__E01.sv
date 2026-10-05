module top;

  if (1) begin : gb
    localparam logic [39:0] GP = 40'hC;
  end
  localparam R = (gb.GP ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
