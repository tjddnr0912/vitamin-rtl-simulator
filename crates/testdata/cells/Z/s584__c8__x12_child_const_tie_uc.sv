module top;
  wire [1:0] o;
  logic t;
  child u(.a(2'd1), .o(o));
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
module child(input logic [1:0] a, output logic [1:0] o);
  always_comb begin
    o = 2'd0;
    unique case (a)
      2'd1: o = 2'd1;
      2'd2: o = 2'd2;
    endcase
  end
endmodule
