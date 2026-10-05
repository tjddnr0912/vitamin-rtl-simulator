module top; function automatic logic [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] f(); return '1; endfunction initial $display("@func %0d %b", $bits(f()), f()); endmodule
