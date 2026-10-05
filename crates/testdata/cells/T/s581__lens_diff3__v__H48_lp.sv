module top;

  localparam R = (4'd12 inside {64'b1?00, 64'b0?11});
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
