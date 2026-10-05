module t;
  localparam [3:0] P4 = 4'b1100;
`ifdef F1
  function automatic logic f(input logic [3:0] a); return a ==? 4'b1?00; endfunction
  localparam L = f(4'b1100);
`elsif F3
  function automatic logic f(); return P4 ==? 4'b1?00; endfunction
  localparam L = f();
`elsif F4
  function automatic logic f(); return (4'd15 + 4'd1) ==? 5'b1?000; endfunction
  localparam L = f();
`elsif F5
  function automatic logic f(input int k); return (4'd15 + 4'd1) ==? 5'b1?000; endfunction
  localparam L = f(0);
`elsif F6
  function automatic logic f(input int k); return P4 ==? 4'b1?00; endfunction
  localparam L = f(0);
`elsif F7
  function automatic logic f(input logic [3:0] a); logic r; r = (a ==? 4'b1?00); return r; endfunction
  localparam L = f(4'b0100);
`elsif F10
  function automatic logic f(); logic r; r = (P4 ==? 4'b1?00); return r; endfunction
  localparam L = f();
`elsif F11
  function automatic logic f(); return (8'sd28 ==? 4'sbx100); endfunction
  localparam L = f();
`elsif F12
  function automatic logic f(); return ({2{2'b10}} ==? 8'b0000_1?10) + (4'sd12 ==? 8'b0000_1?00); endfunction
  localparam L = f();
`endif
  initial begin $display("L=%0d", L); #2 $finish; end
endmodule
