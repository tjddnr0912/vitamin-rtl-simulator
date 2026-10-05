module top;
  localparam int unsigned PU = 32'hFFFF_FFFC;
  localparam R = (PU ==? 'sbx100);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
