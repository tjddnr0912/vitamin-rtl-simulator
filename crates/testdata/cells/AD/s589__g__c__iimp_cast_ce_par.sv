package q;
  localparam int W = 3;
  function automatic int h(input int x); return W'(x); endfunction
endpackage
module top;
  import q::h;
  localparam int W = 7;
  localparam int P = h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
