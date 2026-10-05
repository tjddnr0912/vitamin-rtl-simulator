package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  import q::*;
  localparam int W = 7;
  localparam int P = h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
