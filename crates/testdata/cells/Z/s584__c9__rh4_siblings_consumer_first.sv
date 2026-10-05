module top;
  logic [1:0] d;
  wire [1:0] m;
  wire [1:0] q;
  cons uc(.s(m), .y(q));
  prod up(.a(d), .o(m));
  initial begin d = 2'd1; #1 $display("t=1 q=%0d", q); #1 d = 2'd0; #1 $finish; end
endmodule
module prod(input logic [1:0] a, output logic [1:0] o);
  always_comb o = a;
endmodule
module cons(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    y = 2'd0;
    unique case (s)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
endmodule
