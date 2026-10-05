module top;
  wire [1:0] w;
  logic [1:0] y;
  wire [1:0] q;
  logic t;
  assign w = 2'd1;
  always_comb y = w;
  child u(.a(y), .o(q));
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t q=%b y=%b", $time, q, y);
  initial #1 begin $display("e t=%0t q=%b y=%b", $time, q, y); $finish; end
endmodule
module child(input logic [1:0] a, output logic [1:0] o);
  always_comb o = ~a;
endmodule
