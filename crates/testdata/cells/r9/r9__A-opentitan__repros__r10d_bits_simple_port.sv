module m (input logic [11:0] x, output logic [7:0] o);
  localparam int W = $bits(x);
  assign o = 8'(W);
endmodule
module t;
  logic [7:0] o;
  m u (.x(12'h0), .o(o));
  initial begin #1 $display("A o=%0d", o); $finish; end
endmodule
