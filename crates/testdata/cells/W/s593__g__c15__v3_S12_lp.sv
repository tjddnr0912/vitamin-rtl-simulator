module top;
  typedef struct packed signed {logic [59:0] a; logic [3:0] b;} spk;
  localparam spk SK = '{a: 60'hFFF_FFFF_FFFF_FFFF, b: 4'hC};
  localparam R = (SK ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
