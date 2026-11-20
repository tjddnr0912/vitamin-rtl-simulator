module top; logic [99:0] a; function automatic logic f(); return a <= 'z; endfunction initial $display("%b", f()); endmodule
