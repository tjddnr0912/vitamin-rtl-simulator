module top;
  localparam logic [67:0] P68H = 68'h1_0000_0000_0000_000C;
  localparam R = ($unsigned(P68H) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
