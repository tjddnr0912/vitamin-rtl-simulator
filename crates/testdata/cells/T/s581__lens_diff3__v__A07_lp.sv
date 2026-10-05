module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  localparam R = ($clog2(P40) ==? 6'b10_0?01);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
