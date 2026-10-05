package a;
  localparam logic [7:0] K = 8'hF0;
  function automatic int g(input int d = $clog2(K[7:4] + 4'd15)); return d; endfunction
endpackage
package b;
  localparam logic [15:0] K = 16'h0010;
  function automatic int h(input int x); return a::g() + x; endfunction
endpackage
module top;
  localparam int P = b::h(0);
  int v;
  initial begin v = b::h(0); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
