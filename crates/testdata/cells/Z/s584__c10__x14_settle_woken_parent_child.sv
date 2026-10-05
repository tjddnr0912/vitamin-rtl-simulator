module top;
  wire [1:0] w;
  logic [1:0] a_o;
  assign w = 2'd1;
  always_comb a_o = w;
  child u(.s(a_o), .k(1'b1));
  initial begin #1 $display("t=1 y=%0d", u.y); #1 $finish; end
endmodule
module child(input logic [1:0] s, input logic k);
  logic [1:0] y;
  always_comb begin
    y = 2'd0;
    unique case (s & {k, k})
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
endmodule
