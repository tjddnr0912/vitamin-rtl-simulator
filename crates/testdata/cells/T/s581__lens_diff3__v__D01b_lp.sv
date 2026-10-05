module top;

  localparam R = ("AB" ==? 16'h41_4?);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
