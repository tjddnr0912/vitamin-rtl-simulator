module top;
  wire [1:0] w;
  wire w2;
  logic [1:0] a_o;
  wire [1:0] ao_w;
  logic [1:0] y;
  assign w = 2'd1;
  assign w2 = 1'b1;
  assign ao_w = a_o;
  always_comb begin
    y = 2'd0;
    unique case (ao_w & {w2, w2})
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  always_comb a_o = w;
  initial begin #1 $display("t=1 y=%0d", y); #1 $finish; end
endmodule
