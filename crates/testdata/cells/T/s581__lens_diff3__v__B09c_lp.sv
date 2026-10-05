module top;
  typedef logic [39:0] t40;
  localparam t40 PTF = 40'h0C;
  localparam R = (PTF ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
