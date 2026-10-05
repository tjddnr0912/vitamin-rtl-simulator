module top;
  localparam logic [63:0] P64 = 64'hC;
  localparam R = (P64 ==? 'bx100);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
