module R2;
  localparam logic [((4'bx100 ==? 4'b1?00) | 1'b0):0] P = '1;
  function automatic logic [((4'bx100 ==? 4'b1?00) | 1'b0):0] fr(); return '1; endfunction
  typedef logic [((4'bx100 ==? 4'b1?00) | 1'b0):0] t_t;
  t_t tv;
  initial begin $display("R2 P=%0d fr=%0d t=%0d", $bits(P), $bits(fr()), $bits(tv)); #1 $finish; end
endmodule
