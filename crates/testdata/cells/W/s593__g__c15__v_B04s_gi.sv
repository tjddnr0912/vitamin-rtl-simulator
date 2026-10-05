module top #(parameter type T = logic signed [63:0], parameter T P = -64'sd4);

  if (P ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
