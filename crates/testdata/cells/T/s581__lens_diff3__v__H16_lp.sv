module top;

  localparam R = (4'd12 ==? 64'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
