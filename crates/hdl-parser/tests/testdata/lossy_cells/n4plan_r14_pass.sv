module top; logic [99:0] a; logic b; always_comb b = (a <= 'z) ? 1'b1 : 1'b0; endmodule
