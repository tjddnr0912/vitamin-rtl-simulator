module top;
  localparam logic [39:0] P40F = 40'h0C;
  localparam R = ($unsigned(P40F) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
