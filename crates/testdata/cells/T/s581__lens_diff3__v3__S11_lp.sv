module top;
  typedef struct packed {logic signed [63:0] a; logic [3:0] b;} sts;
  localparam sts SG = '{a: -64'sd4, b: 4'h0};
  localparam R = (SG.a ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
