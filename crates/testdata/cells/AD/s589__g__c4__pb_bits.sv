package q;
  localparam logic [3:0] W = 4'd3;
  function automatic logic [$bits(W)-1:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam logic [7:0] W = 8'd7;
  int v;
  initial begin v = q::h(1000); #1 $display("v=%0d", v); $finish; end
endmodule
