module B5;
  logic [($isunknown(4'bx100 ==? 4'b1?00) - 1):0] v;
  initial begin $display("B5 bits=%0d", $bits(v)); #1 $finish; end
endmodule
