package q;
  localparam int W = 3;
  function automatic logic [31:0] h(input int x); return {W{1'b1}}; endfunction
endpackage
module top;
  localparam logic [31:0] P = q::h(0);
  initial begin #1 $display("P=%h", P); $finish; end
endmodule
