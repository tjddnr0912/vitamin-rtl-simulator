module top;
  typedef logic [39:0] t40;
  localparam t40 PT = 40'h10_0000_000C;
  localparam R = (PT ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
