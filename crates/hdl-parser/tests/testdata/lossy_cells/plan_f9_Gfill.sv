module t #(parameter logic [3:0] B = 4'd0)(); initial #1 $display("A b=%b", B); initial #100 $finish; endmodule
