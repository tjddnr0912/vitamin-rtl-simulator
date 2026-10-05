module B2;
  logic [((4'bx100 ==? 4'b1?00) !== 1'bx):0] v;
  initial begin $display("B2 bits=%0d", $bits(v)); #1 $finish; end
endmodule
