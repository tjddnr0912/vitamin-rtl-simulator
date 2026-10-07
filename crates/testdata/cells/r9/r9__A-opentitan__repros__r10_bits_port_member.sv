typedef struct packed { logic [5:0] src; logic [1:0] sz; logic [31:0] d; } h_t;
module m (input h_t h, output logic [7:0] o);
  localparam int IW = $bits(h.src);
  localparam int DW = $bits(h.d);
  assign o = 8'(IW * 100 + DW);
endmodule
module t;
  h_t h; logic [7:0] o;
  m u (.h(h), .o(o));
  initial begin #1 $display("A o=%0d", o); $finish; end
endmodule
