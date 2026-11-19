module t #(parameter logic [99:0] W = 100'd0)(); initial #1 $display("A w=%h", W); initial #100 $finish; endmodule
