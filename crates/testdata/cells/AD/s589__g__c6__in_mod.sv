module top;
  function logic [3:0] h(input int x); return x; endfunction
  logic [3:0] v; logic [7:0] a = 8'd200;
  always_comb v = h(a);
  initial begin #1 $display("v=%0d", v); $finish; end
endmodule
