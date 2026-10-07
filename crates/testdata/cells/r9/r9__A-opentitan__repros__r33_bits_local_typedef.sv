module m #(parameter int DW = 5) (output logic [7:0] o);
  typedef struct packed { logic w; logic [DW-1:0] d; } req_t;
  localparam int ADW = $bits(req_t);
  assign o = 8'(ADW);
endmodule
module t;
  logic [7:0] a, b;
  m u0 (.o(a));
  m #(.DW(9)) u1 (.o(b));
  initial begin #1 $display("A a=%0d b=%0d", a, b); $finish; end
endmodule
