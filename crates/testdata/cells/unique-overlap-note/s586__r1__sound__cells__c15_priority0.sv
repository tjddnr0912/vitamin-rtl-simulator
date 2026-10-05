module t; logic [1:0] r = 0; int y = 5;
  initial begin priority0 case (r) 2'd1: y = 1; endcase $display("y=%0d", y); #10 $finish; end
endmodule
