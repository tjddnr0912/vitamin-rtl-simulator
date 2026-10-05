module top;
  localparam logic [64:0] P65H = 65'h0_8000_0000_0000_000C;
  localparam R = (P65H ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
