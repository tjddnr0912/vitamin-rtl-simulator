typedef logic [3:0] let;
module sub(input let p); initial #1 $display("B %0d", p); endmodule
module t; logic [3:0] a = 4'd3; sub u(a); initial #2 $finish; endmodule
