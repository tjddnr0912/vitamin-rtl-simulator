module top;

  localparam R = (4'bx100 inside {4'b1?00, 4'bx100});
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
