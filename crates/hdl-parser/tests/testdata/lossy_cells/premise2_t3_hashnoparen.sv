module sub #(parameter logic [7:0] W = 8'd1); initial #1 $display("A w=%b", W); endmodule
module t; sub #8'b1x01_0000 u(); initial #100 $finish; endmodule
