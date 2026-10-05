package q;
  localparam int W = 3;
  function automatic logic [31:0] h(input int x); return {(W*2){1'b1}}; endfunction
endpackage
module top;
  localparam int W = 7;
  logic [31:0] v;
  initial begin v = q::h(0); $display("v=%h", v); #1 $finish; end
endmodule
