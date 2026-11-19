module sub #(parameter logic [3:0] B = 4'd0)(); initial #1 $display("A b=%b", B); endmodule
module t; sub #(.B('x)) u(); initial #100 $finish; endmodule
