package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int W = 7;
  initial begin #1 $display("B=%0d", $bits(q::h(0))); $finish; end
endmodule
