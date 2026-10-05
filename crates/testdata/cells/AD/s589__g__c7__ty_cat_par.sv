package q;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int W = 7;
  localparam logic [31:0] P = {q::h(1000), 4'h0};
  initial begin #1 $display("P=%h", P); $finish; end
endmodule
