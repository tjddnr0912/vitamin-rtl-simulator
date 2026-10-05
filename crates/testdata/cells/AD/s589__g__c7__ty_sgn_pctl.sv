package q;
  localparam int W = 3;
  function automatic logic signed [W:0] h(input int x); return x; endfunction
endpackage
module top;
  
  localparam int P = q::h(15) + 0;
  localparam P2 = q::h(15);
  initial begin #1 $display("P=%0d P2=%0d b=%0d", P, P2, $bits(P2)); $finish; end
endmodule
