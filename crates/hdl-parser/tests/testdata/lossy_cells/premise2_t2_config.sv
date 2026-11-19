module sub #(parameter logic [7:0] W = 8'd1); initial #1 $display("A w=%b", W); endmodule
module t; sub u(); initial #100 $finish; endmodule
config cfg; design t; instance t.u use #(.W(8'b1x01_0000)); endconfig
