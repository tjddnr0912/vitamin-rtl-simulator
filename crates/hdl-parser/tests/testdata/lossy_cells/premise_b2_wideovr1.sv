module s #(parameter logic [99:0] O = 100'd0)(); initial #1 $display("A o=%h", O); endmodule
module t; s #(.O('1)) u(); initial #100 $finish; endmodule
