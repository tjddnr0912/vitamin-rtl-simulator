module t;
  localparam [3:0] P4 = 4'b1100;
`ifdef F2
  function automatic logic f(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  localparam L = f(4'b1100);
`elsif F8
  function automatic logic f(); return P4 inside {4'b1?00}; endfunction
  localparam L = f();
`elsif F9
  function automatic logic f(input int k); return P4 inside {4'b1?00}; endfunction
  localparam L = f(0);
`elsif F13
  function automatic logic f(); return ({2{2'b10}} inside {8'b0000_1?10}); endfunction
  localparam L = f();
`endif
  initial begin $display("L=%0d", L); #2 $finish; end
endmodule
