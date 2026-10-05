module top;
  localparam logic [63:0] P64 = 64'hC;
  localparam R = ((P64 << 1) ==? 5'b1?000);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
