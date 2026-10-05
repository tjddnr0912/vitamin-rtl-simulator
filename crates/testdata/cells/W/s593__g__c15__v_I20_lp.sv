module top;

  localparam R = (4'd12 !=? 'bx100);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
