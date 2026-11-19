module t #(parameter logic [7:0] B = 8'd0)(); initial #1 $display("A b=%b", B); initial #100 $finish; endmodule
