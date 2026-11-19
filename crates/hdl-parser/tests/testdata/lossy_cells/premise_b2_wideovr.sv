module s #(parameter logic [99:0] W = 100'd0, parameter logic [99:0] O = 100'd0)(); initial #1 $display("A w=%h o=%h", W, O); endmodule
module t; s #(.W('x), .O('1)) u(); initial #100 $finish; endmodule
