module top;
  wire [1:0] s;
  logic [1:0] y;
  child u(.o(s));
  always_comb begin
    y = 2'd0;
    unique case (s)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial begin #1 $display("t=1 y=%0d", y); #3 $finish; end
endmodule
module child(output logic [1:0] o);
  logic [1:0] a = 2'd1;
  always_comb o = a;
endmodule
