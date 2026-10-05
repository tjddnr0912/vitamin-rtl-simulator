module top;
  localparam longint unsigned PLU = 64'hFFFF_FFFF_FFFF_FFFC;
  if (PLU !=? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
