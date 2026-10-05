module top;

  localparam R = (4'd12 inside {4'bx100});
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
