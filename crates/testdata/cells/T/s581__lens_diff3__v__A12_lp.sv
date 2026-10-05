module top;
  localparam logic [67:0] P68F = 68'hC;
  localparam R = ($unsigned(P68F) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
