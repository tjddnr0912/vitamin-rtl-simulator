module B6;
  logic [(((4'bx100 ==? 4'b1?00) || 1'b1) - 1):0] v;
  initial begin $display("B6 bits=%0d", $bits(v)); #1 $finish; end
endmodule
