typedef logic [3:0] let;
module t;
  initial begin : blk let y; y = 4'd9; $display("C %0d", y); $finish; end
endmodule
