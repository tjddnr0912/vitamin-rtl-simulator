package q;
  localparam int W = 7;
  function automatic int unsigned h(input logic [W:0] a);
    h = 32'hffffffff;
  endfunction
endpackage
module top;
  localparam longint P = q::h(0);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #50 $finish;
endmodule
