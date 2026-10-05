module top;
  logic [1:0] d;
  wire [1:0] s;
  logic [1:0] y;
  child u(.a(d), .o(s));
  always_comb begin
    y = 2'd0;
    unique case (s)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial begin d = 2'd1; #2 d = 2'd0; #1 d = 2'd1; #1 $finish; end
  initial #1 $display("t=1 y=%0d", y);
endmodule
module child(input logic [1:0] a, output logic [1:0] o);
  always_comb o = a;
endmodule
