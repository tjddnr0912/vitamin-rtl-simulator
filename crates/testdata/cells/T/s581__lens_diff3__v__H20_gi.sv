module top;

  if ((64'hFFFF_FFFF_FFFF_FFFF + 64'd13) ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
