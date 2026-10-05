localparam logic [3:0] U = 4'ha;
package q;
  function automatic logic [7:0] h(input logic [3:0] x); return {U, x}; endfunction
endpackage
module top;
  localparam logic [7:0] P = q::h(4'h5);
  logic [7:0] v;
  initial begin v = q::h(4'h5); #1 $display("P=%h v=%h", P, v); $finish; end
  initial #50 $finish;
endmodule
