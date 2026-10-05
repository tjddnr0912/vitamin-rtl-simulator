localparam int UW = 4;
package q;
  function automatic logic [7:0] h(input logic [7:0] x); return 8'(UW'(x)); endfunction
endpackage
module top;
  localparam logic [7:0] P = q::h(8'hff);
  logic [7:0] v;
  initial begin v = q::h(8'hff); #1 $display("P=%h v=%h", P, v); $finish; end
  initial #50 $finish;
endmodule
