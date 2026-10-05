module top;
  localparam logic [63:0] P64 = 64'hC;
  localparam R = ({1'b0, P64} ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
