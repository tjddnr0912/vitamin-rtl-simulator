package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  import q::h;
  initial begin #1 $display("B=%0d", $bits(h(0))); $finish; end
endmodule
