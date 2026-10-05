module top;

  localparam int R = $bits(4'd12 == 4'd12);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
