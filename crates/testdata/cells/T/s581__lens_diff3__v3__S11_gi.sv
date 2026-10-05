module top;
  typedef struct packed {logic signed [63:0] a; logic [3:0] b;} sts;
  localparam sts SG = '{a: -64'sd4, b: 4'h0};
  if (SG.a ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
