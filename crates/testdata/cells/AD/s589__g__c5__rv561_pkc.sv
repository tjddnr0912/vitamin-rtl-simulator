package q;
  localparam logic [15:0] C = 16'h0123;
  function automatic logic [31:0] g(input int a); return {C, C}; endfunction
endpackage
module top;
  localparam logic [7:0] C = 8'hEE;
  localparam logic [31:0] P = q::g(0);
  logic [31:0] v;
  initial begin v = q::g(0); #1 $display("P=%h v=%h", P, v); $finish; end
endmodule
