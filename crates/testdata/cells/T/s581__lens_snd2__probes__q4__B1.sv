module B1;
  logic [((4'bx100 ==? 4'b1?00) && 1'b0):0] v;
  initial begin $display("B1 bits=%0d", $bits(v)); #1 $finish; end
endmodule
