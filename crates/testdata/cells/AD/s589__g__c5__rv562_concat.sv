package q;
  localparam real R = 3.0;
  function automatic logic [15:0] g(input int a); return {8'(int'(R)), 8'h0}; endfunction
endpackage
module top;
  localparam real R = 9.0;
  localparam logic [15:0] P = q::g(0);
  logic [15:0] v;
  initial begin v = q::g(0); #1 $display("P=%h v=%h", P, v); $finish; end
endmodule
