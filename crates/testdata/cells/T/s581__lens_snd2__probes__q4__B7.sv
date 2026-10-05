module B7;
  logic [((4'bx100 ==? 4'b1?00) ^ (4'bx100 ==? 4'b1?00)):0] v;
  initial begin $display("B7 bits=%0d", $bits(v)); #1 $finish; end
endmodule
