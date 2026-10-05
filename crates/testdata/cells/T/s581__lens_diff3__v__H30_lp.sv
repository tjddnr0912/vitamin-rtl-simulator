module top;
  localparam logic [64:0] P65 = 65'hC;
  localparam R = ((P65 >> 1) ==? 4'b0?10);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
