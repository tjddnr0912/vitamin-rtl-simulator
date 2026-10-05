module top;
  localparam logic [1:0][39:0] PP = {40'h10_0000_000C, 40'h1};
  localparam R = (PP[1][39:0] ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
