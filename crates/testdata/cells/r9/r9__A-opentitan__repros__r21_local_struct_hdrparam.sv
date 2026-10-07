module m #(parameter int AW = 3) (input logic [AW+3:0] in, output logic [7:0] o);
  localparam int WW = AW;
  typedef struct packed { logic [3:0] mask; logic [WW-1:0] woff; } req_t;
  req_t r;
  assign r = in;
  assign o = {r.mask[2], 3'b0, 4'(r.woff)};
endmodule
module t;
  logic [7:0] o2, o5;
  m #(.AW(2)) u2 (.in(6'b110110), .o(o2));
  m #(.AW(5)) u5 (.in(9'b0100_10101), .o(o5));
  initial begin #1 $display("A o2=%h o5=%h", o2, o5); $finish; end
endmodule
