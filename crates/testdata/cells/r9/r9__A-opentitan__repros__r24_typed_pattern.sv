module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  function automatic s_t f(logic [3:0] x); return s_t'{x, ~x}; endfunction
  s_t r;
  initial begin r = f(4'h3); #1 $display("A r=%h", r); $finish; end
endmodule
