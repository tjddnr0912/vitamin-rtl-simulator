module B9;
  logic [(((4'bx100 ==? 4'b1?00) === 1'bx) + 1):0] v;
  initial begin $display("B9 bits=%0d", $bits(v)); #1 $finish; end
endmodule
