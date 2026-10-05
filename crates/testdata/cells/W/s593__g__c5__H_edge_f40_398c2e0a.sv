module m #(parameter type T = logic [7:0], parameter T X = 8'h2c) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
