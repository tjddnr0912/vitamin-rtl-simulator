module top;
  typedef struct packed signed {logic [59:0] a; logic [3:0] b;} spk;
  localparam spk SK = '{a: 60'hFFF_FFFF_FFFF_FFFF, b: 4'hC};
  if (SK ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
