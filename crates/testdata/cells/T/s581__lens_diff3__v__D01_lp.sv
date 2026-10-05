module top;

  localparam R = ("A" ==? 8'b0100_0?01);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
