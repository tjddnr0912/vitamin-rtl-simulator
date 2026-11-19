module s #(parameter logic [99:0] W = 100'd0)(); initial #1 $display("A w=%h", W); endmodule
module t; s u(); defparam u.W = 'z; initial #100 $finish; endmodule
