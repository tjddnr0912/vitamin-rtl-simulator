typedef struct packed { logic [5:0] src; logic [1:0] sz; logic [31:0] d; } h_t;
module m (input h_t h, output logic [7:0] o);
  localparam int W = $bits(h);
  assign o = 8'(W);
endmodule
module t;
  h_t h; logic [7:0] o;
  m u (.h(h), .o(o));
  initial begin #1 $display("A o=%0d", o); $finish; end
endmodule
