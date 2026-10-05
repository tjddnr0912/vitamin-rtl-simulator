module B8;
  logic [((4'bx100 inside {4'b1?00}) && 1'b0):0] v;
  initial begin $display("B8 bits=%0d", $bits(v)); #1 $finish; end
endmodule
