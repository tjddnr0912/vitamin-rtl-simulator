typedef struct packed { logic [((4'bx100 ==? 4'b1?00) | 1'b0):0] f; } s_t;
module R1;
  localparam logic [((4'bx100 ==? 4'b1?00) | 1'b0):0] P = '1;
  s_t s;
  function automatic logic [((4'bx100 ==? 4'b1?00) | 1'b0):0] fr(); return '1; endfunction
  typedef logic [((4'bx100 ==? 4'b1?00) | 1'b0):0] t_t;
  t_t tv;
  initial begin $display("R1 P=%0d s=%0d fr=%0d t=%0d", $bits(P), $bits(s), $bits(fr()), $bits(tv)); #1 $finish; end
endmodule
