package q;
  localparam logic [79:8] K = 72'h616263646566676869;
  function automatic logic [7:0] h(input logic [7:0] d = K[79:72]); return d; endfunction
endpackage
module top;
  localparam logic [79:8] K = 72'h717273747576777879;
  localparam logic [7:0] P = q::h();
  logic [7:0] v;
  initial begin v = q::h(); #1 $display("P=%h v=%h", P, v); $finish; end
endmodule
