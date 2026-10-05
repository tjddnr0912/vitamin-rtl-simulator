module top;

  localparam R = ((4'bx100 ==? 4'b1?00) || 1'b1);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
