package q;
  localparam logic [79:8] K = 72'h036263646566676869;
  function automatic logic [K[79:72]:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam logic [79:8] K = 72'h077273747576777879;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
