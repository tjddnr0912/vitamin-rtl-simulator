package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int W = 7;
  localparam P = q::h(1000);
  initial begin #1 $display("P=%0d b=%0d", P, $bits(P)); $finish; end
endmodule
