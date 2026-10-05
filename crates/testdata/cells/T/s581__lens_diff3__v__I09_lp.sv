module top;
  localparam logic [3:0] PX = 4'b1x00;
  localparam R = (PX ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
